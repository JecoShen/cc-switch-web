declare module "@platform/bootstrap" {
  export interface ConfigLoadErrorPayload {
    path?: string;
    error?: string;
    kind?: string;
    db_version?: number;
    supported_version?: number;
  }

  export function handleFatalConfigLoadError(
    payload: ConfigLoadErrorPayload | null,
  ): Promise<void>;
}
