declare namespace vehicle {
  function listen(
    ip: string,
    port: number
  ): {
    accept: () => Promise<{ serve: (callback: (req: string) => string) => void }>;
    port: string;
    ip: string;
  };

  function writeFile(path: string, contents: string): Promise<void>;

  function readFile(path: string): Promise<string>;

  function removeFile(path: string): Promise<void>;

  function setTimeout(callback: () => void, timeout: number): Promise<void>;
}
