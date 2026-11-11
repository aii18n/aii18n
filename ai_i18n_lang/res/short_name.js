// 去掉区域 / 文字等括号补充，保留简洁语言名
export const shortName = (name) => name.replace(/\s*[（(][^）)]*[）)]\s*$/, '').trim();
