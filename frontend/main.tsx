import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Toaster } from "@/components/ui/toast";
import { TooltipProvider } from "@/components/ui/tooltip";
import { LibraryProvider } from "@/lib/library";
import App from "@/app";
import "@/styles.css";

const root = document.getElementById("root");
if (!root) throw new Error("Application root is missing");
createRoot(root).render(
  <StrictMode>
    <TooltipProvider>
      <Toaster>
        <LibraryProvider>
          <App />
        </LibraryProvider>
      </Toaster>
    </TooltipProvider>
  </StrictMode>,
);
