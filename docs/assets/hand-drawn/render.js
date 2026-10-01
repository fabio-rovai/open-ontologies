// Renders the hand-drawn README animation from a scene file.
//
//   NODE_PATH=<dir holding playwright> node docs/assets/hand-drawn/render.js \
//       knowledge-graph.scene.json knowledge-graph.webp [--fps 15] [--width 1280]
//   ... render.js <scene> <out.png> --still <seconds>    one frame
//   ... render.js <scene> --prove-gate                   the layout gate must fire
//   ... render.js - logo.png --page owl.html --still 0   the owl
//
// Paths are relative to docs/assets. Exits 1, and writes nothing, if the page
// reports a layout error (a label that overlaps another or leaves the frame)
// or a script error. Needs ffmpeg built with libwebp, and webpmux.
const {chromium}=require('playwright');
const http=require('http'),fs=require('fs'),path=require('path'),os=require('os');
const {execFileSync}=require('child_process');
const ROOT=path.resolve(__dirname,'..');
const argv=process.argv.slice(2);
const opt=(k,d)=>{const i=argv.indexOf(k);return i>=0?argv.splice(i,2)[1]:d;};
const PROVE=argv.includes('--prove-gate'); if(PROVE) argv.splice(argv.indexOf('--prove-gate'),1);
const FPS=Number(opt('--fps',15)), WIDTH=Number(opt('--width',1280)), STILL=opt('--still',null);
const PAGE=opt('--page','hero.html');
const [scene,out]=argv;
const MIME={'.html':'text/html','.js':'text/javascript','.json':'application/json','.ttf':'font/ttf'};
(async()=>{
  const srv=http.createServer((q,r)=>{const f=path.join(ROOT,decodeURIComponent(q.url.split('?')[0]));
    if(!f.startsWith(ROOT)||!fs.existsSync(f)||fs.statSync(f).isDirectory()){r.writeHead(404);return r.end();}
    r.writeHead(200,{'Content-Type':MIME[path.extname(f)]||'application/octet-stream'});r.end(fs.readFileSync(f));}).listen(0);
  const b=await chromium.launch(); const pg=await b.newPage({viewport:{width:1920,height:1080}});
  const errs=[]; pg.on('pageerror',e=>errs.push(e.message)); pg.on('console',m=>{if(m.type()==='error')errs.push(m.text());});
  await pg.goto(`http://localhost:${srv.address().port}/hand-drawn/${PAGE}?scene=../${scene}${PROVE?'&probe=1':''}`);
  await pg.waitForFunction('window.READY===true||window.BOOTERR',{timeout:60000});
  const boot=await pg.evaluate('window.BOOTERR'); if(boot) errs.push(boot);
  const L=await pg.evaluate('window.LAYOUT_ERRORS||[]');
  // --prove-gate: a collision is planted, and the gate must report it. A layout
  // check that has never been seen to fail is not known to be a check.
  if(PROVE){ const fired=L.some(e=>e.includes('a planted collision'));
    console.log(fired?'gate fired on the planted collision':'GATE DID NOT FIRE'); await b.close(); srv.close(); process.exit(fired?0:1); }
  if(L.length||errs.length){ console.error('LAYOUT:',L,'\nERRORS:',errs); await b.close(); srv.close(); process.exit(1); }
  const grab=async(t,file)=>{ const d=await pg.evaluate(t=>{if(window.drawAt)window.drawAt(t);return document.getElementById('c').toDataURL('image/png');},t);
    fs.writeFileSync(file,Buffer.from(d.split(',')[1],'base64')); };
  if(STILL!==null){ await grab(Number(STILL),path.resolve(ROOT,out)); }
  else {
    const cycle=await pg.evaluate('window.CYCLE'), n=Math.round(cycle*FPS);
    const tmp=fs.mkdtempSync(path.join(os.tmpdir(),'oo-hero-'));
    for(let f=0;f<n;f++) await grab(f/FPS,path.join(tmp,`f${String(f).padStart(4,'0')}.png`));
    const dst=path.resolve(ROOT,out), ext=path.extname(dst);
    const args=['-y','-loglevel','error','-framerate',String(FPS),'-i',path.join(tmp,'f%04d.png'),'-vf',`scale=${WIDTH}:-2:flags=lanczos`];
    if(ext==='.webp') args.push('-c:v','libwebp_anim','-lossless','0','-quality','72','-compression_level','6','-loop','0');
    else args.push('-c:v','libx264','-pix_fmt','yuv420p','-crf','20','-movflags','+faststart');
    execFileSync('ffmpeg',[...args,dst]);
    fs.rmSync(tmp,{recursive:true});
    // The animation is a raster, so no test can read a number off it. What a test
    // CAN check is which scene drew it: the scene file's digest is stamped into
    // the WebP, and tests/knowledge_graph_asset_test.rs compares it with the
    // scene in the repository, which is itself compared with the SVG.
    if(ext==='.webp'){
      const sha=require('crypto').createHash('sha256').update(fs.readFileSync(path.resolve(ROOT,scene))).digest('hex');
      const xmp=path.join(os.tmpdir(),`oo-hero-${process.pid}.xmp`);
      fs.writeFileSync(xmp,`<x:xmpmeta xmlns:x="adobe:ns:meta/"><oo:scene xmlns:oo="https://open-ontologies.org/ns/asset#">scene-sha256=${sha}</oo:scene></x:xmpmeta>`);
      execFileSync('webpmux',['-set','xmp',xmp,dst,'-o',dst]); fs.rmSync(xmp);
    }
    console.log(`wrote ${out}: ${n} frames at ${FPS} fps, ${(fs.statSync(dst).size/1e6).toFixed(2)} MB`);
  }
  await b.close(); srv.close();
})();
