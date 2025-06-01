export const throwIfTrue = (condition: boolean, exception: Error) => {
  if (condition) {
    throw exception;
  }
};
