export const MAX_PROMPT_TAGS = 50;

export const DEFAULT_PROMPT_TAGS = [
  "用一句话讲个笑话",
  "今天天气怎样？一句话答",
  "给我一个英文单词",
  "用十字内问候我",
  "给一句早安文案",
  "给一句晚安文案",
  "说一个冷知识",
  "推荐一个变量名",
  "给一个函数名建议",
  "生成一个短标题",
  "写一句状态提示",
  "给一个测试用例名",
  "用一句话总结日志",
  "给一句鼓励的话",
  "给一个颜色名称",
  "用一句话解释缓存",
  "用一句话解释队列",
  "用一句话解释重试",
  "用一句话解释超时",
  "用一句话解释 429",
  "给一个接口名称",
  "写一句错误提示",
  "给一个英文短句",
  "用一句话解释令牌",
  "用一句话解释代理",
  "给一个分支名",
  "写一句按钮文案",
  "给一个字段名",
  "讲一个双关笑话",
  "给一句产品提示",
  "用一句话解释心跳",
  "给一个任务标题",
  "写一句空状态文案",
  "给一个状态标签",
  "用一句话解释限流",
  "给一个命名建议",
  "写一句加载文案",
  "给一句周五问候",
  "用一句话解释守护",
  "给一个短别名",
  "写一句成功提示",
  "写一句失败提示",
  "用一句话解释路由",
  "给一个轻量回复",
];

const LEGACY_PROMPT_TAGS = ["只回复 OK", "请只回复 OK", "hi", "ping", "请回复 ready"];
const SHORT_PROMPT_TAGS = [
  "ok",
  "hi",
  "ping",
  "pong",
  "ack",
  "yes",
  "go",
  "up",
  "on",
  "run",
  "rdy",
  "chk",
  "stat",
  "live",
  "beat",
  "tick",
  "tap",
  "echo",
  "noop",
  "test",
  "mark",
  "trace",
  "node",
  "edge",
  "route",
  "gw",
  "api",
  "cc",
  "ar",
  "keep",
  "pulse",
  "warm",
  "wake",
  "link",
  "path",
  "hold",
  "sync",
  "green",
  "ready",
  "ok?",
  "ping?",
  "1",
  "2",
  "3",
];

export function normalizePromptTags(tags: string[]) {
  const seen = new Set<string>();
  const normalized: string[] = [];

  for (const tag of tags) {
    const value = tag.trim();
    if (!value || seen.has(value)) continue;
    seen.add(value);
    normalized.push(value);
    if (normalized.length >= MAX_PROMPT_TAGS) break;
  }

  return normalized;
}

export function promptTagsForUi(tags: string[]) {
  const normalized = normalizePromptTags(tags);
  return sameTags(normalized, LEGACY_PROMPT_TAGS) || sameTags(normalized, SHORT_PROMPT_TAGS)
    ? DEFAULT_PROMPT_TAGS
    : normalized;
}

function sameTags(left: string[], right: string[]) {
  return left.length === right.length && left.every((tag, index) => tag === right[index]);
}
