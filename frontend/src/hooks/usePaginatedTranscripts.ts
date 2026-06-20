import { useState, useCallback, useRef, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Transcript, MeetingMetadata, PaginatedTranscriptsResponse, TranscriptSegmentData } from "@/types";

const DEFAULT_PAGE_SIZE = 100;

interface UsePaginatedTranscriptsProps {
    meetingId: string | null;
    /** Optional initial timestamp (in seconds) from URL for loading the correct page */
    initialTimestamp?: number;
}

interface UsePaginatedTranscriptsReturn {
    metadata: MeetingMetadata | null;
    segments: TranscriptSegmentData[];
    transcripts: Transcript[];
    isLoading: boolean;
    isLoadingMore: boolean;
    hasMore: boolean;
    totalCount: number;
    loadedCount: number;
    error: string | null;

    // Actions
    loadMore: () => Promise<void>;
    loadUntilTranscript: (target: { transcriptId?: string | null; audioStartTime?: number | null }) => Promise<boolean>;
    reset: () => void;
    refetch: () => Promise<void>;
}

/**
 * Convert Transcript array to TranscriptSegmentData for virtualized display
 */
function convertTranscriptsToSegments(transcripts: Transcript[]): TranscriptSegmentData[] {
    return transcripts.map(t => ({
        id: t.id,
        timestamp: t.audio_start_time ?? 0,
        endTime: t.audio_end_time,
        text: t.text,
        confidence: t.confidence,
    }));
}

export function usePaginatedTranscripts({
    meetingId,
    initialTimestamp,
}: UsePaginatedTranscriptsProps): UsePaginatedTranscriptsReturn {
    const [metadata, setMetadata] = useState<MeetingMetadata | null>(null);
    const [transcripts, setTranscripts] = useState<Transcript[]>([]);
    const [totalCount, setTotalCount] = useState(0);
    const [isLoading, setIsLoading] = useState(true);
    const [isLoadingMore, setIsLoadingMore] = useState(false);
    const [hasMore, setHasMore] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const offsetRef = useRef(0);
    const transcriptsRef = useRef<Transcript[]>([]);
    const loadedMeetingIdRef = useRef<string | null>(null);
    const isLoadingRef = useRef(false);
    const lastLoadTimeRef = useRef(0); // Debounce protection

    useEffect(() => {
        transcriptsRef.current = transcripts;
    }, [transcripts]);

    // Reset state when meeting changes
    const reset = useCallback(() => {
        setMetadata(null);
        setTranscripts([]);
        setTotalCount(0);
        setIsLoading(true);
        setIsLoadingMore(false);
        setHasMore(false);
        setError(null);
        offsetRef.current = 0;
    }, []);

    // Load meeting metadata
    const loadMetadata = useCallback(async (): Promise<MeetingMetadata | null> => {
        if (!meetingId) return null;

        try {
            const data = await invoke<MeetingMetadata>('api_get_meeting_metadata', {
                meetingId,
            });
            setMetadata(data);
            return data;
        } catch (err) {
            console.error('Failed to load meeting metadata:', err);
            setError('Failed to load meeting details');
            return null;
        }
    }, [meetingId]);

    // Load transcripts at specific offset
    const loadTranscriptsAtOffset = useCallback(async (
        offset: number,
        append: boolean = true
    ): Promise<Transcript[]> => {
        if (!meetingId) return [];

        try {
            const response = await invoke<PaginatedTranscriptsResponse>(
                'api_get_meeting_transcripts',
                {
                    meetingId,
                    limit: DEFAULT_PAGE_SIZE,
                    offset,
                }
            );

            const newTranscripts = response.transcripts;

            if (append) {
                setTranscripts(prev => {
                    // Deduplicate by id
                    const existingIds = new Set(prev.map(t => t.id));
                    const uniqueNew = newTranscripts.filter(t => !existingIds.has(t.id));
                    // Sort by audio_start_time
                    const merged = [...prev, ...uniqueNew].sort((a, b) =>
                        (a.audio_start_time ?? 0) - (b.audio_start_time ?? 0)
                    );
                    transcriptsRef.current = merged;
                    return merged;
                });
            } else {
                transcriptsRef.current = newTranscripts;
                setTranscripts(newTranscripts);
            }

            setHasMore(response.has_more);
            setTotalCount(response.total_count);
            offsetRef.current = offset + newTranscripts.length;

            return newTranscripts;
        } catch (err) {
            console.error('Failed to load transcripts:', err);
            setError('Failed to load transcripts');
            return [];
        }
    }, [meetingId]);

    // Load next page with debounce protection
    const loadMore = useCallback(async () => {
        const now = Date.now();
        // Debounce: require at least 100ms between calls
        if (now - lastLoadTimeRef.current < 100) {
            return;
        }

        if (isLoadingRef.current || !hasMore || !meetingId || isLoading) return;

        lastLoadTimeRef.current = now;
        isLoadingRef.current = true;
        setIsLoadingMore(true);
        try {
            await loadTranscriptsAtOffset(offsetRef.current, true);
        } finally {
            setIsLoadingMore(false);
            isLoadingRef.current = false;
        }
    }, [hasMore, meetingId, loadTranscriptsAtOffset, isLoading]);

    const transcriptMatchesTarget = useCallback((
        transcript: Transcript,
        target: { transcriptId?: string | null; audioStartTime?: number | null }
    ) => {
        if (target.transcriptId && transcript.id === target.transcriptId) {
            return true;
        }

        if (typeof target.audioStartTime === 'number' && Number.isFinite(target.audioStartTime)) {
            return Math.abs((transcript.audio_start_time ?? 0) - target.audioStartTime) < 0.75;
        }

        return false;
    }, []);

    const loadUntilTranscript = useCallback(async (
        target: { transcriptId?: string | null; audioStartTime?: number | null }
    ): Promise<boolean> => {
        if (!meetingId || (!target.transcriptId && typeof target.audioStartTime !== 'number')) {
            return false;
        }

        if (transcriptsRef.current.some((transcript) => transcriptMatchesTarget(transcript, target))) {
            return true;
        }

        if (isLoadingRef.current) {
            return false;
        }

        isLoadingRef.current = true;
        setIsLoadingMore(true);
        try {
            while (offsetRef.current < totalCount || hasMore) {
                const nextPage = await loadTranscriptsAtOffset(offsetRef.current, true);
                if (nextPage.length === 0) {
                    break;
                }

                if (nextPage.some((transcript) => transcriptMatchesTarget(transcript, target))) {
                    return true;
                }
            }

            return transcriptsRef.current.some((transcript) => transcriptMatchesTarget(transcript, target));
        } finally {
            setIsLoadingMore(false);
            isLoadingRef.current = false;
        }
    }, [hasMore, loadTranscriptsAtOffset, meetingId, totalCount, transcriptMatchesTarget]);

    // Force refetch of data (e.g., after retranscription)
    const refetch = useCallback(async () => {
        if (!meetingId) return;

        reset();
        setIsLoading(true);
        try {
            await loadMetadata();
            await loadTranscriptsAtOffset(0, false);
        } finally {
            setIsLoading(false);
        }
    }, [meetingId, reset, loadMetadata, loadTranscriptsAtOffset]);

    // Initial load
    useEffect(() => {
        if (!meetingId) {
            reset();
            return;
        }

        // Avoid reloading the same meeting
        if (loadedMeetingIdRef.current === meetingId) return;
        loadedMeetingIdRef.current = meetingId;

        reset();

        const loadInitial = async () => {
            setIsLoading(true);
            try {
                await loadMetadata();
                await loadTranscriptsAtOffset(0, false);
            } finally {
                setIsLoading(false);
            }
        };

        loadInitial();
    }, [meetingId, reset, loadMetadata, loadTranscriptsAtOffset]);

    // Convert to segments (memoized)
    const segments = useMemo(() =>
        convertTranscriptsToSegments(transcripts),
        [transcripts]
    );

    return {
        metadata,
        segments,
        transcripts,
        isLoading,
        isLoadingMore,
        hasMore,
        totalCount,
        loadedCount: transcripts.length,
        error,
        loadMore,
        loadUntilTranscript,
        reset,
        refetch,
    };
}
