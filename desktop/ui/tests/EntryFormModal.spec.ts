import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import EntryFormModal from "../src/components/EntryFormModal.vue";

const api = vi.hoisted(() => ({
  entryCreate: vi.fn(),
  entryEdit: vi.fn(),
}));

vi.mock("../src/api", () => ({
  entryCreate: api.entryCreate,
  entryEdit: api.entryEdit,
}));

const initial = {
  title: "GitHub",
  username: "me",
  password: "s3cret",
  url: "https://github.com",
  notes: null,
  otp: null,
  custom_fields: [{ name: "api_key", value: "k-9", protected: false }],
};

describe("EntryFormModal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("创建模式：空标题报错不提交", async () => {
    const wrapper = mount(EntryFormModal, { props: { mode: "create" } });
    await wrapper.find("button.primary").trigger("click");
    expect(api.entryCreate).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain("标题必填");
    wrapper.unmount();
  });

  it("创建模式：otp 非 otpauth:// 报错（对齐 CLI 退出码 4 语义）", async () => {
    const wrapper = mount(EntryFormModal, { props: { mode: "create" } });
    await wrapper.find("#f-title").setValue("X");
    await wrapper.find("#f-otp").setValue("not-otpauth");
    await wrapper.find("button.primary").trigger("click");
    expect(api.entryCreate).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain("otpauth://");
    wrapper.unmount();
  });

  it("创建模式：提交调用 entryCreate 并上抛 saved(null, title)", async () => {
    api.entryCreate.mockResolvedValue(undefined);
    const wrapper = mount(EntryFormModal, { props: { mode: "create" } });
    await wrapper.find("#f-title").setValue(" New Entry ");
    await wrapper.find("#f-password").setValue("p1");
    await wrapper.find("button.primary").trigger("click");
    await flushPromises();
    expect(api.entryCreate).toHaveBeenCalledWith(
      expect.objectContaining({ title: "New Entry", password: "p1", custom_fields: [] }),
    );
    expect(wrapper.emitted("saved")?.[0]).toEqual([null, "New Entry"]);
    wrapper.unmount();
  });

  it("编辑模式：预填 + custom_fields 原样回传（保真口径）", async () => {
    api.entryEdit.mockResolvedValue(undefined);
    const wrapper = mount(EntryFormModal, { props: { mode: "edit", initial } });
    expect((wrapper.find("#f-title").element as HTMLInputElement).value).toBe("GitHub");
    expect((wrapper.find("#f-password").element as HTMLInputElement).value).toBe("s3cret");
    // 标题改名 + 密码修改，custom_fields 未渲染编辑控件但随草稿回传
    await wrapper.find("#f-title").setValue("GitHub2");
    await wrapper.find("#f-password").setValue("p2");
    await wrapper.find("button.primary").trigger("click");
    await flushPromises();
    expect(api.entryEdit).toHaveBeenCalledWith(
      "GitHub",
      expect.objectContaining({
        title: "GitHub2",
        password: "p2",
        custom_fields: initial.custom_fields,
      }),
    );
    expect(wrapper.emitted("saved")?.[0]).toEqual(["GitHub", "GitHub2"]);
    wrapper.unmount();
  });

  it("编辑模式不渲染自定义字段编辑控件（S1b-1 口径）", () => {
    const wrapper = mount(EntryFormModal, { props: { mode: "edit", initial } });
    expect(wrapper.text()).not.toContain("api_key");
    wrapper.unmount();
  });
});
