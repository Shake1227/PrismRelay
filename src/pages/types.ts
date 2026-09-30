import type { BackupManifest, Page, ScanReport, ScanRequest } from "../models";

export interface WorkspaceProps {
  scan: ScanReport;
  backups: BackupManifest[];
  request: ScanRequest;
  onError: (error: unknown) => void;
  onNotice: (message: string) => void;
  onRefresh: () => Promise<void>;
  navigate: (page: Page) => void;
}
