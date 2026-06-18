"use client";

import { useEffect, useState } from 'react';
import { Download, FileText, Loader2, Settings2 } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import {
  ExportFormat,
  ExportHistoryEntry,
  ExportSections,
  ExportSettings,
  defaultExportSettings,
  exportMeeting,
  getExportHistory,
  getExportSettings,
  updateExportSettings,
} from '@/services/exportService';

interface ExportMeetingDialogProps {
  meetingId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const formatLabels: Record<ExportFormat, string> = {
  markdown: 'Markdown',
  pdf: 'PDF',
  docx: 'DOCX',
};

export function ExportMeetingDialog({
  meetingId,
  open,
  onOpenChange,
}: ExportMeetingDialogProps) {
  const [settings, setSettings] = useState<ExportSettings>(defaultExportSettings);
  const [format, setFormat] = useState<ExportFormat>('markdown');
  const [sections, setSections] = useState<ExportSections>(defaultExportSettings.sections);
  const [destinationDir, setDestinationDir] = useState('');
  const [fileName, setFileName] = useState('');
  const [history, setHistory] = useState<ExportHistoryEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isSavingSettings, setIsSavingSettings] = useState(false);

  useEffect(() => {
    if (!open) return;

    let cancelled = false;
    const load = async () => {
      setIsLoading(true);
      try {
        const [loadedSettings, loadedHistory] = await Promise.all([
          getExportSettings(),
          getExportHistory(meetingId),
        ]);

        if (!cancelled) {
          setSettings(loadedSettings);
          setFormat(loadedSettings.defaultFormat);
          setSections(loadedSettings.sections);
          setDestinationDir(loadedSettings.destinationDir ?? '');
          setFileName(loadedSettings.fileNameTemplate);
          setHistory(loadedHistory);
        }
      } catch (error) {
        console.error('Failed to load export settings:', error);
        toast.error('Failed to load export settings');
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    };

    void load();

    return () => {
      cancelled = true;
    };
  }, [meetingId, open]);

  const updateSection = (key: keyof ExportSections, value: boolean) => {
    setSections((current) => ({ ...current, [key]: value }));
  };

  const handleSaveSettings = async () => {
    setIsSavingSettings(true);
    try {
      const saved = await updateExportSettings({
        ...settings,
        defaultFormat: format,
        sections,
        destinationDir: destinationDir.trim() || null,
        fileNameTemplate: fileName.trim() || defaultExportSettings.fileNameTemplate,
      });
      setSettings(saved);
      toast.success('Export preferences saved');
    } catch (error) {
      console.error('Failed to save export settings:', error);
      toast.error('Failed to save export preferences');
    } finally {
      setIsSavingSettings(false);
    }
  };

  const handleExport = async () => {
    setIsLoading(true);
    try {
      const result = await exportMeeting(meetingId, {
        format,
        sections,
        destinationDir: destinationDir.trim() || null,
        fileName: fileName.trim() || null,
      });
      setHistory(await getExportHistory(meetingId));
      toast.success(`${formatLabels[result.format]} export created`, {
        description: result.filePath,
      });
    } catch (error) {
      console.error('Failed to export meeting:', error);
      toast.error('Failed to export meeting', {
        description: String(error),
      });
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[640px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Download className="h-5 w-5" />
            Export meeting
          </DialogTitle>
          <DialogDescription>
            Create a local export with the meeting metadata, summary, action items, and transcript.
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-5 py-2">
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label>Format</Label>
              <Select value={format} onValueChange={(value) => setFormat(value as ExportFormat)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="markdown">Markdown</SelectItem>
                  <SelectItem value="pdf">PDF</SelectItem>
                  <SelectItem value="docx">DOCX</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label>File name template</Label>
              <Input
                value={fileName}
                onChange={(event) => setFileName(event.target.value)}
                placeholder="{title}-{date}"
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label>Destination folder</Label>
            <Input
              value={destinationDir}
              onChange={(event) => setDestinationDir(event.target.value)}
              placeholder="Meetily exports folder"
            />
          </div>

          <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
            <SectionSwitch label="Metadata" checked={sections.metadata} onChange={(checked) => updateSection('metadata', checked)} />
            <SectionSwitch label="Summary" checked={sections.summary} onChange={(checked) => updateSection('summary', checked)} />
            <SectionSwitch label="Actions" checked={sections.actionItems} onChange={(checked) => updateSection('actionItems', checked)} />
            <SectionSwitch label="Transcript" checked={sections.transcript} onChange={(checked) => updateSection('transcript', checked)} />
          </div>

          <div className="rounded-md border border-gray-200 p-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-sm font-medium">Auto-export after summary</p>
                <p className="text-xs text-gray-500">Uses the selected sections and auto-export format.</p>
              </div>
              <Switch
                checked={settings.autoExportEnabled}
                onCheckedChange={(checked) => setSettings((current) => ({ ...current, autoExportEnabled: checked }))}
              />
            </div>
            <div className="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2">
              <Select
                value={settings.autoExportFormat}
                onValueChange={(value) => setSettings((current) => ({ ...current, autoExportFormat: value as ExportFormat }))}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="markdown">Markdown</SelectItem>
                  <SelectItem value="pdf">PDF</SelectItem>
                  <SelectItem value="docx">DOCX</SelectItem>
                </SelectContent>
              </Select>
              <Button
                type="button"
                variant="outline"
                onClick={handleSaveSettings}
                disabled={isSavingSettings}
              >
                {isSavingSettings ? <Loader2 className="animate-spin" /> : <Settings2 />}
                Save preferences
              </Button>
            </div>
          </div>

          {history.length > 0 && (
            <div className="space-y-2">
              <Label>Recent exports</Label>
              <div className="max-h-28 space-y-2 overflow-y-auto rounded-md border border-gray-200 p-2">
                {history.slice(0, 4).map((entry) => (
                  <div key={`${entry.filePath}-${entry.createdAt}`} className="flex min-w-0 items-center gap-2 text-xs text-gray-600">
                    <FileText className="h-4 w-4 shrink-0" />
                    <span className="font-medium">{formatLabels[entry.format]}</span>
                    <span className="truncate">{entry.filePath}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            Close
          </Button>
          <Button type="button" onClick={handleExport} disabled={isLoading}>
            {isLoading ? <Loader2 className="animate-spin" /> : <Download />}
            Export
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function SectionSwitch({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex items-center justify-between gap-2 rounded-md border border-gray-200 px-3 py-2 text-sm">
      <span>{label}</span>
      <Switch checked={checked} onCheckedChange={onChange} />
    </label>
  );
}
