async function main() {
  console.log(1);
  await new Promise((res) => setTimeout(res, 5));
  console.log(2);

  setTimeout(() => {
    console.log(3);
    setTimeout(async () => {
      console.log(4);
      await new Promise((res) => setTimeout(res, 5));
      console.log(5);

      require("./handler.mjs");

      console.log(6);
    }, 5);
  }, 5);
}

main();
