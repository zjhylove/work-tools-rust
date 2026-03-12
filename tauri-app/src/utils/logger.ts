/**
 * 统一的日志工具
 * 只在开发环境输出日志,生产环境禁用所有日志
 */

/**
 * 开发环境日志 - 只在开发模式输出
 */
export const devLog = (...args: unknown[]) => {
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.log(...args);
  }
};

/**
 * 开发环境错误日志 - 只在开发模式输出
 */
export const devError = (...args: unknown[]) => {
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.error(...args);
  }
};

/**
 * 开发环境警告日志 - 只在开发模式输出
 */
export const devWarn = (...args: unknown[]) => {
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.warn(...args);
  }
};

/**
 * 开发环境调试日志 - 只在开发模式输出
 */
export const devDebug = (...args: unknown[]) => {
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.debug(...args);
  }
};
