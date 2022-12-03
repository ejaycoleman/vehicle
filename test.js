console.log("Hello", "world!");
console.error("Hello", "error!");

const path = "./file.txt";

await vehicle.writeFile(path, "Write to file.");

try {
  const contents = await vehicle.readFile(path);
  console.log("Read from a file", contents);
} catch (err) {
  console.error("Error reading file", path, err);
}

console.log("Removing file", path);
vehicle.removeFile(path);
console.log("File removed");
