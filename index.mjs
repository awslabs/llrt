const iterations = 10 * 1000 * 1000;
let count = 0;
for (let i = 0; i < iterations; i++) {
  setTimeout(() => {
    count++;
    if (i === iterations - 1) {
      console.log(count);
    }
  }, 0);
}

// // let interval = setInterval(() => {
// //   console.log("==============BANAN======");
// //   clearInterval(interval);
// // }, 1000);

// async function main() {
//   const start = Date.now();
//   let count = 1;
//   await new Promise((resolve) => {
//     let interval = setInterval(() => {
//       if (count == 2) {
//         clearInterval(interval);
//         return;
//       }
//       count++;
//     }, 5);
//     setTimeout(resolve, 20);
//   });
//   const end = Date.now();
//   console.log("123");

//   // await new Promise((resolve, reject) => {
//   //   setTimeout(() => {
//   //     console.log("========= 1");
//   //     resolve();
//   //   }, 10);
//   // });
//   // console.log("continue 1");
//   // await new Promise((resolve, reject) => {
//   //   setTimeout(() => {
//   //     console.log("======== 2");
//   //     resolve();
//   //   }, 10);
//   // });
//   // console.log("continue 2");
//   // setTimeout(() => {
//   //   console.log("======= 3");
//   // }, 10);
// }

// main().catch(console.error);

// // setTimeout(() => {
// //   setTimeout(() => {
// //     console.log("2");
// //   }, 2000);
// // }, 1000);
