/* Vendored unchanged from the Tesseract hand-drawn film engine (video-v2),
   so the README art is drawn with the same primitives as the films. */
/* Hand-drawn film engine. Everything is drawn at runtime on a canvas:
   wobbly ink lines, watercolour washes, paper grain, characters, camera. */
export const W=1920,H=1080;
/* Two visual languages over one set of assets. 'paper' is the hand-drawn look:
   wobbling ink, watercolour bleed, paper grain. 'flat' is the Kurzgesagt-style
   language: hard edges, solid fills, rounded geometry, vivid colour on a dark
   ground, no texture at all. Themeing the PRIMITIVES means all 122 props and 11
   backdrops change language without being redrawn. */
const THEMES={
 paper:{ paper:'#F6F1E4', ink:'#2B2A28', ink2:'#4A4742',
   teal:'#2C7773', tealL:'#7FB8B2', amber:'#C87A33', amberL:'#E9B784',
   grey:'#9A968C', greyL:'#CFC9BC', red:'#B4483C', sky:'#DDE7E4',
   text:'#2B2A28', textDim:'#5A5A63', wobble:1, bleed:1, grain:1, outline:1 },
 flat:{  paper:'#121C2E', ink:'#0A1220', ink2:'#223busy',
   teal:'#31D4C4', tealL:'#7FE9DE', amber:'#FFB347', amberL:'#FFD79B',
   grey:'#6E819C', greyL:'#2B3A54', red:'#FF6B8A', sky:'#1B2942',
   text:'#EEF4FA', textDim:'#9FB2CC', wobble:0, bleed:0, grain:0, outline:0 }
};
THEMES.flat.ink2='#223250';
/* 'studio': the technical-premium language, calibrated from the anime.js design
   system — a warm near-black ramp rather than navy, vivid warm accents, a grotesk
   for language and a monospace for anything verifiable, tight geometry, no texture. */
THEMES.studio={
  paper:'#252423', surface:'#2A2928', raise:'#302E2D', edge:'#3F3E3D',
  ink:'#F6F4F2', ink2:'#A9A5A0',
  teal:'#35D6C4', tealL:'#7FEADE', amber:'#FFA828', amberL:'#FFCC2A',
  grey:'#8C8781', greyL:'#3A3938', red:'#FF4B4B', sky:'#2A2928',
  text:'#F6F4F2', textDim:'#A9A5A0',
  wobble:0, bleed:0, grain:0, outline:0, tight:1
};
export const PAL={...THEMES.paper};
export let THEME='paper';
export function setTheme(name){
  THEME = THEMES[name]?name:'paper';
  Object.assign(PAL, THEMES[THEME]);
  PAPER=null;
}
let BOIL=true;
export function setBoil(v){BOIL=v;}

/* deterministic per-frame noise so the "line boil" is stable within a frame */
export function mulberry(a){return function(){a|=0;a=a+0x6D2B79F5|0;let t=Math.imul(a^a>>>15,1|a);
  t=t+Math.imul(t^t>>>7,61|t)^t;return((t^t>>>14)>>>0)/4294967296;};}
let RND=mulberry(1), FRAME=0, BOILNOW=false;
/* Jitter is a pure function of an element's GEOMETRY, not of the frame number, so a
   line that is not moving is drawn identically every frame and does not crawl.
   Boil (frame-varying noise) is opt-in per element via withBoil(). */
function h32(args){let h=2166136261>>>0;
  for(const a of args){const s=String(a);
    for(let i=0;i<s.length;i++){h^=s.charCodeAt(i);h=Math.imul(h,16777619)>>>0;}}
  return h>>>0;}
function seedFor(...args){
  const g=args.map(v=>typeof v==='number'?Math.round(v/3):v);
  RND=mulberry((h32(g)^(BOILNOW?Math.imul(FRAME,2654435761):0))>>>0);
}
export function seedFrame(f){FRAME=f;BOILNOW=false;RND=mulberry(12345);}
export function withBoil(fn){
  const was=BOILNOW; BOILNOW=BOIL; try{fn();} finally{BOILNOW=was;}
}
export const rnd=()=>RND();
const jit=a=>(rnd()-0.5)*2*a;

/* ---------- ink ---------- */
function wobble(pts,amp){
  const o=[];
  for(let i=0;i<pts.length;i++){
    const edge=(i===0||i===pts.length-1)?0.35:1;
    o.push([pts[i][0]+jit(amp*edge),pts[i][1]+jit(amp*edge)]);
  }
  return o;
}
function resample(pts,step){
  const o=[];
  for(let i=0;i<pts.length-1;i++){
    const [x0,y0]=pts[i],[x1,y1]=pts[i+1];
    const d=Math.hypot(x1-x0,y1-y0),n=Math.max(1,Math.round(d/step));
    for(let k=0;k<n;k++)o.push([x0+(x1-x0)*k/n,y0+(y1-y0)*k/n]);
  }
  o.push(pts[pts.length-1]);return o;
}
function spline(ctx,p){
  ctx.moveTo(p[0][0],p[0][1]);
  for(let i=1;i<p.length-1;i++){
    const xc=(p[i][0]+p[i+1][0])/2, yc=(p[i][1]+p[i+1][1])/2;
    ctx.quadraticCurveTo(p[i][0],p[i][1],xc,yc);
  }
  if(p.length>1)ctx.lineTo(p[p.length-1][0],p[p.length-1][1]);
}
function cutPath(p,frac){
  if(frac>=1) return p;
  if(frac<=0) return [];
  let total=0; const seg=[];
  for(let i=0;i<p.length-1;i++){const d=Math.hypot(p[i+1][0]-p[i][0],p[i+1][1]-p[i][1]);
    seg.push(d); total+=d;}
  let want=total*frac, out=[p[0]];
  for(let i=0;i<seg.length;i++){
    if(want<=0) break;
    if(seg[i]<=want){ out.push(p[i+1]); want-=seg[i]; }
    else { const k=want/seg[i];
      out.push([p[i][0]+(p[i+1][0]-p[i][0])*k, p[i][1]+(p[i+1][1]-p[i][1])*k]); want=0; }
  }
  return out;
}
export function ink(ctx,pts,{w=6,c=PAL.ink,amp=2.4,close=false,passes=2,alpha=1,draw=1}={}){
  if(draw<=0) return;
  if(!PAL.wobble){ amp=0; passes=1; }
  /* Flat languages drop the sketchy contour around a FILLED shape, but line-only
     props (ladder, key, gear spokes) are nothing but strokes, so dropping them
     erased whole assets. Keep the stroke, let the palette recolour it. */
  if(!PAL.outline && c===PAL.ink && w<=7){ alpha*=0.85; }
  seedFor('ink',pts[0][0],pts[0][1],pts[pts.length-1][0],pts[pts.length-1][1],pts.length,w);
  const base=resample(pts,26);
  ctx.save();ctx.lineCap='round';ctx.lineJoin='round';ctx.globalAlpha=alpha;
  for(let k=0;k<passes;k++){
    let p=wobble(base,amp);
    if(draw<1){ p=cutPath(p,draw); if(p.length<2) break; close=false; }
    ctx.beginPath();spline(ctx,p);if(close)ctx.closePath();
    ctx.strokeStyle=c;ctx.lineWidth=w*(k?0.62:1);ctx.globalAlpha=alpha*(k?0.5:1);
    ctx.stroke();
  }
  ctx.restore();
}
/* ---------- watercolour ---------- */
export function wash(ctx,pts,c,{alpha=0.5,spread=9,layers=3,close=true,draw=1}={}){
  if(draw<=0) return;
  if(!PAL.bleed){ alpha=1; spread=0; layers=1; }
  if(draw<1) alpha*=draw;
  seedFor('wash',pts[0][0],pts[0][1],pts.length,c);
  ctx.save();ctx.globalAlpha=alpha/layers*1.5;ctx.fillStyle=c;
  for(let k=0;k<layers;k++){
    const p=wobble(resample(pts,34),spread*(0.5+k*0.5));
    ctx.beginPath();spline(ctx,p);if(close)ctx.closePath();ctx.fill();
  }
  ctx.restore();
}
export function blob(ctx,cx,cy,r,c,{alpha=0.5,n=13,wob=0.22,layers=3}={}){
  if(!PAL.bleed){ alpha=1; wob=0; layers=1; n=40; }
  seedFor('blob',cx,cy,r,c);
  const pts=[];
  for(let i=0;i<n;i++){const a=i/n*Math.PI*2;const rr=r*(1+jit(wob));
    pts.push([cx+Math.cos(a)*rr,cy+Math.sin(a)*rr*0.94]);}
  pts.push(pts[0]);wash(ctx,pts,c,{alpha,spread:r*0.05,layers});
  return pts;
}
export function ring(ctx,cx,cy,r,{w=6,c=PAL.ink,amp=2.2,n=22}={}){
  if(!PAL.wobble){ amp=0; n=48; }
  seedFor('ring',cx,cy,r,w);
  const pts=[];
  for(let i=0;i<=n;i++){const a=i/n*Math.PI*2;pts.push([cx+Math.cos(a)*r,cy+Math.sin(a)*r]);}
  ink(ctx,pts,{w,c,amp});
}
export function rect(ctx,x,y,w,h,{stroke=PAL.ink,fill=null,lw=6,amp=2.4}={}){
  const p=[[x,y],[x+w,y],[x+w,y+h],[x,y+h],[x,y]];
  if(fill)wash(ctx,p,fill,{alpha:0.62,spread:5});
  if(stroke)ink(ctx,p,{w:lw,c:stroke,amp,close:false});
}
/* ---------- paper ---------- */
let PAPER=null;
export function paper(ctx){
  if(!PAL.grain){
    ctx.fillStyle=PAL.paper; ctx.fillRect(0,0,W,H);
    const g=ctx.createRadialGradient(W/2,H*0.44,H*0.15,W/2,H*0.5,H*1.05);
    g.addColorStop(0,'rgba(255,255,255,0.055)'); g.addColorStop(1,'rgba(0,0,0,0.20)');
    ctx.fillStyle=g; ctx.fillRect(0,0,W,H); return;
  }
  if(!PAPER){
    PAPER=document.createElement('canvas');PAPER.width=W;PAPER.height=H;
    const g=PAPER.getContext('2d');
    g.fillStyle=PAL.paper;g.fillRect(0,0,W,H);
    const r=mulberry(9);
    const d=g.getImageData(0,0,W,H),a=d.data;
    for(let i=0;i<a.length;i+=4){const n=(r()-0.5)*20;a[i]+=n;a[i+1]+=n;a[i+2]+=n;}
    g.putImageData(d,0,0);
    g.globalAlpha=0.014;g.fillStyle='#8C8474';
    for(let i=0;i<34;i++){const x=r()*W,y=r()*H,rr=140+r()*420;
      g.beginPath();g.ellipse(x,y,rr,rr*0.62,r()*3,0,7);g.fill();}
    g.globalAlpha=1;
  }
  ctx.drawImage(PAPER,0,0);
}
let GRAIN=null;
function grainPlate(){
  if(GRAIN)return GRAIN;
  GRAIN=document.createElement('canvas');GRAIN.width=W;GRAIN.height=H;
  const g=GRAIN.getContext('2d');g.fillStyle='#fff';g.fillRect(0,0,W,H);
  const r=mulberry(23);const d=g.getImageData(0,0,W,H),a=d.data;
  for(let i=0;i<a.length;i+=4){const n=(r()-0.5)*30;a[i]+=n;a[i+1]+=n;a[i+2]+=n;}
  g.putImageData(d,0,0);return GRAIN;
}
export function grainOver(ctx,t){
  if(!PAL.grain) return;
  ctx.save();ctx.globalAlpha=0.07;ctx.globalCompositeOperation='multiply';
  ctx.drawImage(grainPlate(),0,0);
  ctx.restore();
  const g=ctx.createRadialGradient(W/2,H/2,H*0.42,W/2,H/2,H*0.92);
  g.addColorStop(0,'rgba(60,52,38,0)');g.addColorStop(1,'rgba(60,52,38,0.18)');
  ctx.fillStyle=g;ctx.fillRect(0,0,W,H);
}
/* ---------- text ---------- */
export function hand(ctx,str,x,y,size,{font='Caveat',c=PAL.ink,align='left',rot=0,alpha=1}={}){
  /* type must read against the ground: in the flat theme the ground is dark, so
     anything asking for ink or a muted grey is remapped to the theme's text colours */
  if(PAL.text){
    if(c===PAL.ink||c==='#2B2A28'||c==='#0A1220') c=PAL.text;
    else if(c===PAL.ink2||c===PAL.grey||c===PAL.slate) c=PAL.textDim;
  }
  if(PAL.tight){                      // studio: language in a grotesk, figures in mono
    font = (font==='GochiHand'||/^[\d\s.,:%€$+\-\/]+$/.test(str)) ? 'JetBrainsMono' : 'Inter';
    size = font==='JetBrainsMono' ? size*0.86 : size*0.88;
  }
  seedFor('text',x,y,size,str);
  ctx.save();ctx.globalAlpha=alpha;ctx.translate(x,y);ctx.rotate(rot+jit(0.006));
  ctx.font=`${PAL.tight?600:700} ${size}px "${font}"`;ctx.textAlign=align;ctx.textBaseline='alphabetic';
  ctx.fillStyle=c;ctx.fillText(str,jit(1.2),jit(1.2));ctx.restore();
}
export function underline(ctx,x,y,w,c=PAL.teal,lw=7){
  ink(ctx,[[x,y],[x+w*0.4,y+jit(3)],[x+w,y]],{w:lw,c,amp:2.6});
}
/* ---------- camera ---------- */
export function cam(ctx,{x=0,y=0,z=1,rot=0}={}){
  ctx.translate(W/2,H/2);ctx.scale(z,z);ctx.rotate(rot);ctx.translate(-W/2-x,-H/2-y);
}
/* ---------- character ---------- */
export function person(ctx,x,y,s,{pose='stand',t=0,coat=PAL.teal,skin='#E8C39E',
                                  hair=PAL.ink2,face=1,talk=0}={}){
  const P=v=>[x+v[0]*s,y+v[1]*s];
  const br=Math.sin(t*1.9)*2;                       // breathing
  const A={stand:[[-34,-104],[-42,-52]],point:[[46,-132],[96,-150]],
           up:[[42,-158],[58,-206]],hold:[[-40,-118],[-16,-150]]}[pose]||[[-34,-104],[-42,-52]];
  // legs
  ink(ctx,[P([-8,-86]),P([-22,-44]),P([-26,0])],{w:7*s,c:PAL.ink,amp:2.2});
  ink(ctx,[P([8,-86]),P([20,-44]),P([24,0])],{w:7*s,c:PAL.ink,amp:2.2});
  ink(ctx,[P([-38,2]),P([-18,2])],{w:8*s,c:PAL.ink,amp:1.4});
  ink(ctx,[P([14,2]),P([34,2])],{w:8*s,c:PAL.ink,amp:1.4});
  // coat
  const body=[P([-30,-96+br]),P([-36,-150+br]),P([-24,-176+br]),P([24,-176+br]),
              P([36,-150+br]),P([30,-96+br]),P([-30,-96+br])];
  wash(ctx,body,coat,{alpha:0.66,spread:6});
  ink(ctx,body,{w:6.5*s,c:PAL.ink,amp:2.2});
  // arms
  ink(ctx,[P([-26,-166+br]),P(A[0]),P(A[1])],{w:6.5*s,c:PAL.ink,amp:2.2});
  ink(ctx,[P([26,-166+br]),P([38,-116+br]),P([44,-64+br])],{w:6.5*s,c:PAL.ink,amp:2.2});
  // head
  const hx=x, hy=y+(-214+br)*s, hr=30*s;
  blob(ctx,hx,hy,hr*1.02,skin,{alpha:0.8,layers:2,wob:0.1});
  ring(ctx,hx,hy,hr,{w:6.5*s,c:PAL.ink,amp:1.8});
  ink(ctx,[P([0,-186+br]),P([0,-176+br])],{w:6*s,c:PAL.ink,amp:1.2});
  // hair
  const hp=[[hx-hr,hy-6],[hx-hr*0.8,hy-hr*1.15],[hx,hy-hr*1.35],[hx+hr*0.82,hy-hr*1.1],[hx+hr,hy-4]];
  wash(ctx,hp,hair,{alpha:0.8,spread:5});
  if(face){
    const blink=(Math.sin(t*0.7)>0.985)?0.15:1;
    ctx.save();ctx.fillStyle=PAL.ink;
    ctx.beginPath();ctx.ellipse(hx-10*s,hy-3*s,3.2*s,3.4*s*blink,0,0,7);ctx.fill();
    ctx.beginPath();ctx.ellipse(hx+10*s,hy-3*s,3.2*s,3.4*s*blink,0,0,7);ctx.fill();
    ctx.restore();
    const m=talk?(0.5+Math.abs(Math.sin(t*11))*0.9):0.45;
    ink(ctx,[[hx-8*s,hy+12*s],[hx,hy+(12+6*m)*s],[hx+8*s,hy+12*s]],{w:4*s,c:PAL.ink,amp:1});
  }
}
/* ---------- helpers ---------- */
export const ease=x=>{x=Math.max(0,Math.min(1,x));return x*x*(3-2*x);};
export const eo=x=>{x=Math.max(0,Math.min(1,x));return 1-Math.pow(1-x,3);};
export const lerp=(a,b,k)=>a+(b-a)*k;
