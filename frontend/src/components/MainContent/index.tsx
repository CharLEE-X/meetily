'use client';

import React from 'react';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';

interface MainContentProps {
  children: React.ReactNode;
}

const MainContent: React.FC<MainContentProps> = ({ children }) => {
  const { isCollapsed } = useSidebar();

  return (
    <main 
      className={`min-w-0 flex-1 transition-[margin] duration-300 ease-out ${
        isCollapsed ? 'ml-[4.5rem]' : 'ml-72'
      }`}
    >
      <div className="pl-6">
        {children}
      </div>
    </main>
  );
};

export default MainContent;
