import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import App from "@/app";
import { SidebarProvider } from "@/components/ui/sidebar";
import { Toaster } from "@/components/ui/toast";
import { TooltipProvider } from "@/components/ui/tooltip";
import { LibraryProvider } from "@/lib/library";

import "@/styles.css";

const root = document.querySelector("#root");

if (!root) {
  throw new Error("Application root is missing");
}

createRoot(root).render(
  <StrictMode>
    <TooltipProvider>
      <Toaster>
        <LibraryProvider>
          <SidebarProvider className="h-svh min-h-0 overflow-hidden">
            <App />
          </SidebarProvider>
        </LibraryProvider>
      </Toaster>
    </TooltipProvider>
  </StrictMode>
);
