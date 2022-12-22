((globalThis) => {
  const { core } = Deno;
  const { ops } = core;

  core.initializeAsyncOps();

  function argsToMessage(...args) {
    return args.map((arg) => JSON.stringify(arg)).join(" ");
  }

  globalThis.console = {
    log: (...args) => {
      core.print(`[out]: ${argsToMessage(...args)}\n`, false);
    },
    error: (...args) => {
      core.print(`[err]: ${argsToMessage(...args)}\n`, true);
    },
  };

  const requestBuf = new Uint8Array(64 * 1024);
  const responseBuf = (v) =>
    new Uint8Array(
      `HTTP/1.1 200 OK\r\nContent-Length: ${
        `{"request": "${v}"}`.length
      }\r\nContent-Type: application/json\r\n\r\n{"request": "${v}"}\n`
        .split("")
        .map((c) => c.charCodeAt(0))
    );

  globalThis.vehicle = {
    readFile: (path) => {
      return ops.op_read_file(path);
    },
    writeFile: (path, contents) => {
      return ops.op_write_file(path, contents);
    },
    removeFile: (path) => {
      return ops.op_remove_file(path);
    },
    setTimeout: async (callback, timeout) => {
      await ops.timeout(timeout);
      callback();
    },
    listen: (port) => {
      return ops.op_listen(port);
    },
    accept: (serverRid) => {
      return ops.op_accept(serverRid);
    },
    serve: async (rid) => {
      try {
        while (true) {
          await core.read(rid, requestBuf);

          let readValue = "";
          requestBuf.forEach((a) => {
            if (a !== 0) {
              readValue += String.fromCharCode(a);
            }
          });

          await core.writeAll(rid, responseBuf(readValue));
        }
      } catch (e) {
        if (
          !e.message.includes("Broken pipe") &&
          !e.message.includes("Connection reset by peer")
        ) {
          throw e;
        }
      }
      core.close(rid);
    },
  };
})(globalThis);
