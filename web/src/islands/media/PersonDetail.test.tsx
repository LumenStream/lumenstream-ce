/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { PersonDetail } from "./PersonDetail";

const getPersonMock = vi.fn();
const getPersonItemsMock = vi.fn();

const mockAuthState = {
  ready: true,
  session: {
    token: "token-1",
    user: { Id: "user-1", Name: "tester" },
  },
};

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => mockAuthState,
}));

vi.mock("@/lib/api/items", () => ({
  getPerson: (...args: unknown[]) => getPersonMock(...args),
  getPersonItems: (...args: unknown[]) => getPersonItemsMock(...args),
  buildItemImageUrl: (itemId: string) => `https://image.example.com/${itemId}`,
  buildPersonImageUrl: (personId: string) => `https://person.example.com/${personId}`,
}));

async function flushEffects() {
  await Promise.resolve();
  await Promise.resolve();
}

describe("PersonDetail", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    (globalThis as typeof globalThis & { React?: typeof React }).React = React;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    getPersonMock.mockResolvedValue({
      Id: "person-actor-1",
      Name: "Al Pacino",
      Type: "Person",
      Path: "",
      Overview: "演员简介",
    });

    getPersonItemsMock.mockResolvedValue({
      Items: [
        {
          Id: "movie-2",
          Name: "教父2",
          Type: "Movie",
          Path: "/media/godfather2.strm",
          ProductionYear: 1974,
          People: [
            {
              Id: "person-actor-1",
              Name: "Al Pacino",
              Role: "Michael Corleone",
              Type: "Actor",
            },
          ],
        },
      ],
      TotalRecordCount: 1,
      StartIndex: 0,
    });
  });

  afterEach(() => {
    act(() => {
      root.unmount();
    });
    container.remove();
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = undefined;
    vi.clearAllMocks();
  });

  it("renders person profile and role credits", async () => {
    await act(async () => {
      root.render(<PersonDetail personId="person-actor-1" />);
      await flushEffects();
    });

    expect(getPersonMock).toHaveBeenCalledWith("person-actor-1");
    expect(getPersonItemsMock).toHaveBeenCalledWith(
      "person-actor-1",
      expect.objectContaining({
        includeItemTypes: "Movie,Series,Episode",
        limit: 200,
      })
    );

    const text = container.textContent || "";
    expect(text).toContain("Al Pacino");
    expect(text).toContain("参演作品");
    expect(text).toContain("教父2");
    expect(text).toContain("饰演 Michael Corleone");

    const itemLink = container.querySelector("a[href='/app/item/movie-2']");
    expect(itemLink).not.toBeNull();
  });
});
