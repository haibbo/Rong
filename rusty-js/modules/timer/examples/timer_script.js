setTimeout(() => {
    print("Timeout triggered after 2 seconds");
}, 2000);

let count = 0;
const intervalId = setInterval(() => {
    count++;
    print("Interval triggered every 1 second. Count: " + count);
    if (count >= 5) {
        clearInterval(intervalId);
        print("Interval cancelled after 5 times");
    }
}, 1000); 