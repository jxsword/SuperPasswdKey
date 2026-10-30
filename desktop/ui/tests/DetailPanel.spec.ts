import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import DetailPanel from "../src/components/DetailPanel.vue";

const api = vi.hoisted(() => ({
  entryGet: vi.fn(),
  entryTotp: vi.fn(),
  clipCopy: vi.fn(),
}));

vi.mock("../src/api", () => ({
  entryGet: api.entryGet,
  entryTotp: api.entryTotp,
  clipCopy: api.clipCopy,
}));

describe("DetailPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  it("渲染条目字段（密码掩码）", async () => {
    api.entryGet.mockResolvedValue({
      title: "GitHub",
      username: "me",
      password: "s3cret",
      url: "https://github.com",
      notes: "some notes",
      otp: null,
    });
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    expect(wrapper.text()).toContain("GitHub");
    expect(wrapper.text()).toContain("me");
    expect(wrapper.text()).toContain("●●●●●●●●");
    expect(wrapper.text()).not.toContain("s3cret"); // 密码不回显明文
    expect(wrapper.text()).toContain("some notes");
  });

  it("有 OTP 时轮询动态码", async () => {
    api.entryGet.mockResolvedValue({
      title: "GitHub",
      username: "me",
      password: "x",
      url: null,
      notes: null,
      otp: "otpauth://totp/GitHub?secret=JBSWY3DPEHPK3PXP",
    });
    api.entryTotp.mockResolvedValue("123456");
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    expect(api.entryTotp).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain("123456");
    // 30s 轮询
    vi.advanceTimersByTime(30_000);
    await flushPromises();
    expect(api.entryTotp).toHaveBeenCalledTimes(2);
    wrapper.unmount();
  });

  it("复制按钮调用 clipCopy", async () => {
    api.entryGet.mockResolvedValue({
      title: "GitHub",
      username: "me",
      password: "s3cret",
      url: null,
      notes: null,
      otp: null,
    });
    api.clipCopy.mockResolvedValue(undefined);
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    const copyBtns = wrapper.findAll("button.copy");
    await copyBtns[0].trigger("click");
    await flushPromises();
    expect(api.clipCopy).toHaveBeenCalledWith("s3cret");
    expect(wrapper.text()).toContain("已复制");
    wrapper.unmount();
  });

  it("锁定语义错误展示", async () => {
    api.entryGet.mockRejectedValue(new Error("会话已锁定，请重新解锁"));
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    expect(wrapper.text()).toContain("会话已锁定");
    wrapper.unmount();
  });
});
