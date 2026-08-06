import { test, expect } from "@playwright/test";
test("real worker resumes a pending HTA evaluator fiber",async({page})=>{await page.goto("/rust/web/hta-browser.html");await expect.poll(()=>page.evaluate(()=>window.htaSmoke?.then(String))).toBe("42");await page.evaluate(()=>window.htaContext.close());});

test("browser promise provider follows the real Chromium event loop",async({page})=>{
  await page.goto("/rust/web/hta-browser.html");
  const result=await page.evaluate(async()=>{
    const {BrowserPromiseProvider}=await import("/rust/web/hta.js");
    const provider=new BrowserPromiseProvider(),events=[];
    const source=provider.run(()=>{events.push("run");return 21;});
    const chained=provider.then(source,value=>{events.push("then");return value*2;});
    events.push("sync");
    return {value:await chained,events,native:chained instanceof Promise};
  });
  expect(result).toEqual({value:42,events:["sync","run","then"],native:true});
});
