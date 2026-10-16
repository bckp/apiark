import {describe,it,expect} from "vitest";
import {extractPathVariables,resolvePathVariables,parseUrlParams,buildUrlWithParams} from "@/lib/url-params";
describe("shared URL helpers",()=>{
  it("recognizes complete path segments without ports, schemes or query values",()=>{
    const url="http://localhost:8080/:id/:other-id?q=:ignored#ignored";
    expect(extractPathVariables(url)).toEqual(["id","other-id"]);
    expect(resolvePathVariables(url,{id:"a b/č","other-id":"1"})).toBe("http://localhost:8080/a%20b%2F%C4%8D/1?q=:ignored#ignored");
  });
  it("keeps duplicates, plus semantics, malformed percent escapes and fragments",()=>{
    const parsed=parseUrlParams("https://example.test?x=a+b&x=%2B&bad=%no#fragment");
    expect(parsed.params.map(p=>p.value)).toEqual(["a b","+","%no"]);
    expect(buildUrlWithParams(parsed.baseUrl,parsed.params)).toBe("https://example.test?x=a+b&x=%2B&bad=%25no#fragment");
  });
});
