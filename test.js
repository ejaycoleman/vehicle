// console.log("Hello", "world!");
// console.error("Hello", "error!");

// const path = "./file.txt";

// await vehicle.writeFile(path, "Write to file.");

// try {
//   const contents = await vehicle.readFile(path);
//   console.log("Read from a file", contents);
// } catch (err) {
//   console.error("Error reading file", path, err);
// }

// console.log("Removing file", path);
// vehicle.removeFile(path);
// console.log("File removed");

// vehicle.setTimeout(() => console.log("After 3 seconds"), 3000);
// console.log("test");

// vehicle.websocket("4000", () => console.log("connect"));

async function main() {
  const { port, accept } = vehicle.listen(3000);
  console.log(`http_bench_ops listening on http://127.0.0.1:${port}`);

  while (true) {
    await accept((req) => {
      console.log(req);
      const res = "res";
      return res;
    });
  }
}

main();
