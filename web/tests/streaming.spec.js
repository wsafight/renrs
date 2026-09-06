import {test, expect} from '@playwright/test';
import {advance} from './reading-helpers.js';

test('streaming video renders, pauses, restores and releases its source', async({page}, testInfo) => {
  test.skip(!process.env.RENRS_STREAM_TEST, 'Requires the streaming fixture at /stream/');
  await page.setViewportSize({width:390,height:844});
  const errors=[]; page.on('pageerror', error=>errors.push(error.message));
  await page.goto('/stream/');
  await expect(page.locator('#text')).toContainText('Media checkpoint');
  await advance(page);
  const video=page.locator('#video');
  await expect(video).toBeVisible();
  await expect.poll(()=>video.evaluate(video=>video.currentTime)).toBeGreaterThan(0.15);
  expect(await video.evaluate(video=>{
    const canvas=document.createElement('canvas'); canvas.width=160; canvas.height=90;
    const context=canvas.getContext('2d');context.drawImage(video,0,0,160,90);
    const pixels=context.getImageData(0,0,160,90).data;
    return pixels.some((value,index)=>index%4!==3&&value>60);
  })).toBe(true);
  await page.getByRole('button',{name:'Saves',exact:true}).click();
  await page.getByRole('button',{name:'Save',exact:true}).first().click();
  const paused=await video.evaluate(video=>video.currentTime);
  await page.waitForTimeout(500);
  expect(await video.evaluate(video=>video.currentTime)).toBeCloseTo(paused,2);
  await page.getByRole('button',{name:'Close',exact:true}).click();
  await page.screenshot({path:testInfo.outputPath('stream.png'),fullPage:true});
  await expect(page.locator('#text')).toContainText('Video finished');
  await page.getByRole('button',{name:'Saves',exact:true}).click();
  page.once('dialog',dialog=>dialog.accept());
  await page.getByRole('button',{name:'Load',exact:true}).first().click();
  await expect(video).toBeVisible();
  const restored=await video.evaluate(video=>video.currentTime);
  expect(restored).toBeGreaterThanOrEqual(paused-0.08);expect(restored).toBeLessThan(paused+0.5);
  await expect(page.locator('#text')).toContainText('Video finished');
  await expect(video).toBeHidden();expect(await video.getAttribute('src')).toBeNull();
  expect(errors).toEqual([]);
});
