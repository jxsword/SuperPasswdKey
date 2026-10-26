// 表单探测纯函数（SP-F-704，design_docs/12 §2）：吃描述符不吃 DOM，vitest 直测。
// 规则：唯一可见密码字段；用户名=密码前同表单第一个可见输入（autocomplete=username 优先，
// 排除 search/query/captcha 语义）；多密码字段=非单页登录，本期拒绝。

export interface InputDescriptor {
  /** DOM 顺序号 */
  index: number;
  type: string;
  name: string;
  id: string;
  autocomplete: string;
  visible: boolean;
  /** 所属表单序号（-1 = 无表单） */
  formSeq: number;
}

export interface LoginFields {
  usernameIndex: number | null;
  passwordIndex: number;
  totpIndex: number | null;
}

const USERNAME_TYPES = new Set(["text", "email", "tel", ""]);
const EXCLUDE_NAME = /search|query|captcha/i;

export function detectLoginFields(inputs: InputDescriptor[]): LoginFields | null {
  const passwords = inputs.filter((i) => i.type === "password" && i.visible);
  if (passwords.length === 0) return null; // 非登录页
  if (passwords.length > 1) return null; // 注册/改密页（design_docs/12 §2 规则 5）
  const password = passwords[0];

  // TOTP 可选填充目标（规则 3）
  const totp = inputs.find(
    (i) => i.visible && i.autocomplete === "one-time-code",
  );
  const totpIndex = totp && totp.index !== password.index ? totp.index : null;

  // 用户名候选：密码之前、同表单（密码无表单则不限）、类型白名单
  const candidates = inputs.filter(
    (i) =>
      i.index < password.index &&
      i.visible &&
      USERNAME_TYPES.has(i.type) &&
      !EXCLUDE_NAME.test(i.name) &&
      !EXCLUDE_NAME.test(i.id) &&
      (password.formSeq === -1 || i.formSeq === password.formSeq) &&
      i.autocomplete !== "one-time-code",
  );
  if (candidates.length === 0) {
    return { usernameIndex: null, passwordIndex: password.index, totpIndex };
  }
  // autocomplete=username 强制优先（规则 3）；否则 DOM 序第一个
  const username =
    candidates.find((i) => i.autocomplete === "username") ?? candidates[0];
  return {
    usernameIndex: username.index,
    passwordIndex: password.index,
    totpIndex,
  };
}
