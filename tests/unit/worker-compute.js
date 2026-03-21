// Computation worker - performs calculations
self.onmessage = function(event) {
  const data = event.data;

  if (data.type === "sum") {
    // Calculate sum from 1 to max
    let sum = 0;
    for (let i = 1; i <= data.max; i++) {
      sum += i;
    }
    postMessage(sum);
  }
};
