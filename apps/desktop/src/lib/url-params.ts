import type {KeyValuePair} from "@apiark/types";

export function extractPathVariables(url: string): string[] {
  return [...new Set(url.split(/[?#]/,1)[0].split("/").filter(p=>/^:[\w-]+$/.test(p)).map(p=>p.slice(1)))];
}

export function resolvePathVariables(url: string, values: Record<string,string>): string {
  const split=url.search(/[?#]/);const index=split<0?url.length:split;
  return url.slice(0,index).split("/").map(p=>p.startsWith(":")&&values[p.slice(1)] ? encodeURIComponent(values[p.slice(1)]) : p).join("/")+url.slice(index);
}

export function parseUrlParams(url:string):{baseUrl:string;params:KeyValuePair[]} {
  const hash=url.indexOf("#");const fragment=hash<0?"":url.slice(hash);const beforeHash=hash<0?url:url.slice(0,hash);
  const q=beforeHash.indexOf("?");if(q<0)return {baseUrl:url,params:[]};
  return {baseUrl:beforeHash.slice(0,q)+fragment,params:[...new URLSearchParams(beforeHash.slice(q+1))].map(([key,value])=>({id:crypto.randomUUID(),key,value,enabled:true}))};
}

export function buildUrlWithParams(url:string,params:KeyValuePair[]):string {
  const {baseUrl}=parseUrlParams(url);const hash=baseUrl.indexOf("#");const fragment=hash<0?"":baseUrl.slice(hash);const base=hash<0?baseUrl:baseUrl.slice(0,hash);
  const query=new URLSearchParams(params.filter(p=>p.enabled&&p.key.trim()).map(p=>[p.key,p.value])).toString().replace(/%7B%7B/g,"{{").replace(/%7D%7D/g,"}}");
  return base+(query?`?${query}`:"")+fragment;
}
