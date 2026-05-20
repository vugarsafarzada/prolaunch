export interface ScriptInfo {
  name: string;
  command: string;
  packageManager?: string;
  source?: string;
}

export interface ProjectInfo {
  name: string;
  path: string;
  scripts: ScriptInfo[];
  packageManager: string;
}

export interface LogEvent {
  project_path: string;
  script_name: string;
  line: string;
  is_error: boolean;
}

export interface ProcessEndEvent {
  project_path: string;
  script_name: string;
  exit_code: number | null;
}

export interface Tab {
  id: string;
  project: ProjectInfo;
}

export interface RunningScript {
  project_path: string;
  script_name: string;
  pid: number;
}

export interface LogLine {
  text: string;
  isError: boolean;
  timestamp: number;
  scriptName: string;
}
