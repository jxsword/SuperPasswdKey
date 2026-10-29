// SavePendingStore 纯逻辑单测（SP-F-708）：tab 隔离 / TTL 过期 / confirm 移除 /
// dismiss 忽略本会话不再询问（host 维度）/ 内存纪律（无任何 storage 调用面）。
import { describe, expect, it } from "vitest";
import { PENDING_TTL_MS, SavePendingStore, urlHost } from "../src/background/save-pending";

const LOGIN = { url: "https://ex.com/login", username: "u1", password: "p1" };

function store(now = 1_000_000): { s: SavePendingStore; tick: (ms: number) => void } {
  let t = now;
  const s = new SavePendingStore(() => t);
  return { s, tick: (ms) => (t += ms) };
}

describe("urlHost", () => {
  it("提取 host 并容忍非法 URL", () => {
    expect(urlHost("https://ex.com/login?x=1")).toBe("ex.com");
    expect(urlHost("http://localhost:8081/a")).toBe("localhost:8081");
    expect(urlHost("not-a-url")).toBeNull();
  });
});

describe("SavePendingStore", () => {
  it("capture 后 peek 可见（不移除），confirm 移除", () => {
    const { s } = store();
    s.capture(LOGIN, 7);
    expect(s.peek(7)?.username).toBe("u1");
    expect(s.peek(7)?.username).toBe("u1"); // peek 幂等（锁定重试需要保留）
    const it = s.confirm(7);
    expect(it?.url).toBe("https://ex.com/login");
    expect(s.peek(7)).toBeNull();
    expect(s.confirm(7)).toBeNull();
  });

  it("tab 隔离：别的 tab 查不到", () => {
    const { s } = store();
    s.capture(LOGIN, 7);
    expect(s.peek(8)).toBeNull();
    expect(s.peek(7)).not.toBeNull();
  });

  it("TTL 过期即清（浮层不再出现）", () => {
    const { s, tick } = store();
    s.capture(LOGIN, 7);
    tick(PENDING_TTL_MS + 1);
    expect(s.peek(7)).toBeNull();
    expect(s.confirm(7)).toBeNull();
  });

  it("新捕获覆盖同 tab 旧捕获", () => {
    const { s } = store();
    s.capture(LOGIN, 7);
    s.capture({ url: "https://ex2.com/", username: "u2", password: "p2" }, 7);
    expect(s.peek(7)?.username).toBe("u2");
  });

  it("dismiss：本会话同 host 不再询问", () => {
    const { s } = store();
    s.capture(LOGIN, 7);
    s.dismiss(7);
    expect(s.peek(7)).toBeNull();
    // 同 host 再捕获 → peek 仍为 null（本会话忽略）
    s.capture(LOGIN, 7);
    expect(s.peek(7)).toBeNull();
    // 别的 host 不受影响
    s.capture({ url: "https://other.com/", username: "u", password: "p" }, 7);
    expect(s.peek(7)).not.toBeNull();
    // dismiss 清掉了 pending：确认无残留
    expect(s.confirm(7)?.url).toBe("https://other.com/");
  });

  it("dismiss 忽略按 host 维度（同 host 任意 tab 均忽略）", () => {
    const { s } = store();
    s.capture(LOGIN, 7);
    s.dismiss(7);
    s.capture(LOGIN, 9);
    expect(s.peek(9)).toBeNull();
  });
});
