import { ApiError } from "./api";

/**
 * The sentence to show when a write failed. "Signed in but not allowed"
 * (403) deserves its own explanation; everything else gets the generic one.
 * Pure so every screen words failures the same way and it can be tested.
 */
export function mutationErrorMessage(
  error: unknown,
  forbiddenMessage: string,
): string {
  if (error instanceof ApiError && error.status === 403) {
    return forbiddenMessage;
  }
  return "Something went wrong. Try again.";
}
