// Echo worker - sends back messages with "echo: " prefix
self.onmessage = function(event) {
  postMessage("echo: " + event.data);
};
