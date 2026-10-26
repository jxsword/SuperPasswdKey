// 表单探测规则单测（SP-F-704，design_docs/12 §2 全分支）
import { describe, expect, it } from "vitest";
import type { InputDescriptor } from "../src/content/detect";
import { detectLoginFields } from "../src/content/detect";

let seq = 0;
function input(partial: Partial<InputDescriptor>): InputDescriptor {
  return {
    index: seq++,
    type: "text",
    name: "",
    id: "",
    autocomplete: "",
    visible: true,
    formSeq: 0,
    ...partial,
  };
}

describe("detectLoginFields", () => {
  it("基础登录表单：用户名取密码前第一个可见输入", () => {
    seq = 0;
    const fields = detectLoginFields([
      input({ name: "user" }),
      input({ type: "password", name: "pass" }),
    ]);
    expect(fields).toEqual({
      usernameIndex: 0,
      passwordIndex: 1,
      totpIndex: null,
    });
  });

  it("无密码字段 → null", () => {
    seq = 0;
    expect(detectLoginFields([input({ name: "user" })])).toBeNull();
  });

  it("多密码字段（注册页）→ null", () => {
    seq = 0;
    expect(
      detectLoginFields([
        input({ type: "password" }),
        input({ type: "password" }),
      ]),
    ).toBeNull();
  });

  it("隐藏密码字段不计入", () => {
    seq = 0;
    expect(
      detectLoginFields([
        input({ type: "password", visible: false }),
        input({ type: "password", name: "pw2" }),
      ]),
    ).toMatchObject({ passwordIndex: 1 });
  });

  it("autocomplete=username 跨序优先", () => {
    seq = 0;
    const fields = detectLoginFields([
      input({ name: "nickname" }),
      input({ name: "user", autocomplete: "username" }),
      input({ type: "password" }),
    ]);
    expect(fields).toMatchObject({ usernameIndex: 1 });
  });

  it("排除 search/query/captcha 语义字段", () => {
    seq = 0;
    const fields = detectLoginFields([
      input({ name: "search" }),
      input({ id: "captcha" }),
      input({ name: "user" }),
      input({ type: "password" }),
    ]);
    expect(fields).toMatchObject({ usernameIndex: 2 });
  });

  it("排除非文本类型与类型白名单外输入", () => {
    seq = 0;
    const fields = detectLoginFields([
      input({ type: "search", name: "q" }),
      input({ type: "hidden", name: "token" }),
      input({ name: "user" }),
      input({ type: "password" }),
    ]);
    expect(fields).toMatchObject({ usernameIndex: 2 });
  });

  it("同表单约束：密码有表单时用户名必须同表单", () => {
    seq = 0;
    const fields = detectLoginFields([
      input({ name: "other", formSeq: 1 }),
      input({ name: "user", formSeq: 0 }),
      input({ type: "password", formSeq: 0 }),
    ]);
    expect(fields).toMatchObject({ usernameIndex: 1 });
  });

  it("密码无表单（formSeq=-1）时允许无表单用户名", () => {
    seq = 0;
    const fields = detectLoginFields([
      input({ name: "user", formSeq: -1 }),
      input({ type: "password", formSeq: -1 }),
    ]);
    expect(fields).toMatchObject({ usernameIndex: 0 });
  });

  it("密码前无候选 → usernameIndex=null 仍可填密码", () => {
    seq = 0;
    const fields = detectLoginFields([
      input({ type: "password" }),
      input({ name: "user" }),
    ]);
    expect(fields).toMatchObject({ usernameIndex: null, passwordIndex: 0 });
  });

  it("one-time-code 输入识别为 TOTP 目标", () => {
    seq = 0;
    const fields = detectLoginFields([
      input({ name: "user" }),
      input({ type: "password" }),
      input({ autocomplete: "one-time-code" }),
    ]);
    expect(fields).toMatchObject({ totpIndex: 2 });
  });

  it("otp 输入不作用户名候选", () => {
    seq = 0;
    const fields = detectLoginFields([
      input({ autocomplete: "one-time-code" }),
      input({ name: "user" }),
      input({ type: "password" }),
    ]);
    expect(fields).toMatchObject({ usernameIndex: 1 });
  });
});
