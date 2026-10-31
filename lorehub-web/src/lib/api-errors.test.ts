import { describe, expect, it } from "vitest";
import { ApiError } from "./api";
import { mutationErrorMessage } from "./api-errors";

describe("mutationErrorMessage", () => {
  it("explains a 403 with the caller's own wording", () => {
    const message = mutationErrorMessage(
      new ApiError("POST /x failed: 403", 403),
      "Only an Owner or Admin can do this.",
    );
    expect(message).toBe("Only an Owner or Admin can do this.");
  });

  it("falls back to the generic sentence for other statuses", () => {
    expect(
      mutationErrorMessage(new ApiError("POST /x failed: 500", 500), "no"),
    ).toBe("Something went wrong. Try again.");
  });

  it("falls back for errors that are not ApiErrors (network failure)", () => {
    expect(mutationErrorMessage(new TypeError("Failed to fetch"), "no")).toBe(
      "Something went wrong. Try again.",
    );
    expect(mutationErrorMessage("boom", "no")).toBe(
      "Something went wrong. Try again.",
    );
  });
});
