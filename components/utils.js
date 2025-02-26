export const tagFn = fn => (strings, ...parts) => fn(parts.reduce((tpl, value, i) => `${tpl}${strings[i]}${value}`, '').concat(strings[parts.length]))
export const html = tagFn(s => new DOMParser().parseFromString(`<template>${s}</template>`, 'text/html').querySelector('template'))
export const css = tagFn(s => s)