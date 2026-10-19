# Quitting stops transfers

Quitting Opul stops transfers, and closing the window quits Opul. The app does not keep running with no window, and it does not ask when transfers are still running. The next launch resumes those Downloads. A helper that kept downloading after quit was the alternative, and it was rejected for this version because it would be a second process the person did not open.

## Considered options

A background helper that continues transfers while the window is closed. Rejected for this version.
