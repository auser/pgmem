const timeoutPromise = async ({
  promise,
  timeout = 1000,
  failureMessage
}) => {
  let timeoutHandle;

  const error = new Error(failureMessage || `Timeout of ${timeout}ms exceeded`);
  const timeoutPromise = new Promise((resolve, reject) => {
    timeoutHandle = setTimeout(() => reject(error), timeout);
  });

  const result = await Promise.race([promise, timeoutPromise]);

  clearTimeout(timeoutHandle);

  return result;
};

const sleep = async (timeout = 20) => new Promise((resolve) => setTimeout(resolve, timeout));

module.exports = {
  timeoutPromise,
  sleep
}