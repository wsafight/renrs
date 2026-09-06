import {test,expect} from '@playwright/test';
import {advance} from './reading-helpers.js';
for(const viewport of [{width:1280,height:800},{width:390,height:844}]) {
  test(`story, saves, debugger and layout at ${viewport.width}`,async({page},testInfo)=>{
    await page.setViewportSize(viewport);
    const errors=[];page.on('pageerror',error=>errors.push(error.message));
    await page.goto('/');
    await expect(page.locator('#text')).toContainText('receiver');
    await expect.poll(()=>page.locator('#background').evaluate(image=>image.naturalWidth)).toBeGreaterThan(0);
    await page.screenshot({path:testInfo.outputPath('playing.png'),fullPage:true});
    await page.getByRole('button',{name:'Saves',exact:true}).click();
    await page.getByRole('textbox',{name:'Slot 1 note'}).fill('Browser regression');
    await page.getByRole('button',{name:'Save',exact:true}).first().click();
    await expect(page.getByText('Browser regression',{exact:true})).toBeVisible();
    await page.getByRole('button',{name:'Close',exact:true}).click();
    await advance(page);
    await expect(page.getByRole('button',{name:'Let Mira explain',exact:true})).toBeVisible();
    await page.getByRole('button',{name:'Let Mira explain',exact:true}).click();
    await expect(page.locator('#text')).toContainText('Thank you');
    await page.getByRole('button',{name:'Debugger',exact:true}).click();
    await expect(page.locator('#variables')).toContainText('trust');
    await expect(page.locator('#graph canvas').first()).toBeVisible();
    await expect.poll(() => page.locator('#graph canvas').evaluateAll(canvases => canvases.some(canvas => {
      const pixels = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data;
      let colored = 0;
      for(let index = 3; index < pixels.length; index += 4) if(pixels[index]) colored++;
      return colored > 100;
    }))).toBe(true);
    await page.screenshot({path:testInfo.outputPath('debug.png'),fullPage:true});
    await page.getByRole('button',{name:'Saves',exact:true}).click();
    page.once('dialog',dialog=>dialog.accept());
    await page.getByRole('button',{name:'Load',exact:true}).first().click();
    await expect(page.locator('#text')).toContainText('receiver');
    await page.getByRole('button',{name:'Settings',exact:true}).click();
    await page.getByLabel('High contrast').check();
    await expect(page.locator('body')).toHaveClass(/contrast/);
    await page.screenshot({path:testInfo.outputPath('settings.png'),fullPage:true});
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    expect(errors).toEqual([]);
  });
}

test('instruction breakpoint stops before mutation and exports route coverage', async({page}) => {
  await page.goto('/');
  await expect(page.locator('#text')).toContainText('receiver');
  await page.getByRole('button', {name:'Debugger',exact:true}).click();
  await page.locator('.instruction').filter({hasText:'Set'}).first().getByRole('checkbox').check();
  await advance(page);
  await expect(page.locator('#location')).toContainText('Paused');
  await expect(page.locator('#variables')).not.toContainText('trust');
  await page.getByRole('button', {name:'Step instruction',exact:true}).click();
  await expect(page.locator('#variables')).toContainText('trust');
  await expect(page.locator('#location')).toContainText('Paused');
  await page.getByRole('button', {name:'Resume',exact:true}).click();
  await page.getByRole('button', {name:'Let Mira explain',exact:true}).click();
  const download = page.waitForEvent('download');
  await page.getByRole('button', {name:'Export coverage',exact:true}).click();
  const stream = await (await download).createReadStream();
  let contents = ''; for await (const chunk of stream) contents += chunk.toString();
  const coverage = JSON.parse(contents);
  expect(coverage.choices).toEqual([0]);
  expect(coverage.instructions.length).toBeGreaterThan(4);
  expect(coverage.project_id).toBe('org.renrs.signal-at-dusk');
});

test('collection remains unlocked after a browser restart', async({page}) => {
  await page.goto('/');
  await expect(page.locator('#text')).toContainText('receiver');
  await advance(page);
  await page.getByRole('button', {name:'Let Mira explain',exact:true}).click();
  await advance(page);
  await expect(page.locator('#text')).toContainText('three-note');
  await page.getByRole('button', {name:'Collection',exact:true}).click();
  await expect(page.getByText('A signal at dusk',{exact:true})).toBeVisible();
  await page.reload();
  await expect(page.locator('#text')).toContainText('receiver');
  await page.getByRole('button', {name:'Collection',exact:true}).click();
  await expect(page.getByText('A signal at dusk',{exact:true})).toBeVisible();
});

test('blocked startup music resumes on the first player gesture', async({page}) => {
  await page.addInitScript(() => {
    const play = HTMLMediaElement.prototype.play;
    window.blockedAudio = false;
    window.startedAudio = false;
    HTMLMediaElement.prototype.play = function() {
      if (!window.blockedAudio) {
        window.blockedAudio = true;
        return Promise.reject(new DOMException('Autoplay requires a gesture', 'NotAllowedError'));
      }
      return play.call(this).then(() => { window.startedAudio = true; });
    };
  });
  await page.goto('/');
  await expect.poll(() => page.evaluate(() => window.blockedAudio)).toBe(true);
  await page.getByRole('button',{name:'Continue',exact:true}).click();
  await expect.poll(() => page.evaluate(() => window.startedAudio)).toBe(true);
});
