/* The Open Ontologies owl, drawn by hand in code with the house film engine.

   It replaces a generated raster. The owl perches on one EDGE of a graph, two
   nodes and the line between them, because that is the thing the engine
   judges. Ink, one watercolour accent (the teal of a certified edge) and warm
   paper, so it reads on GitHub's light and dark pages alike: everything sits on
   a paper sticker with an ink rim, and outside the rim is transparent. */
import {PAL,ink,wash,blob,ring,hand,mulberry} from './engine.js';

const S=1024, C=S/2;
const cv=document.getElementById('c'), ctx=cv.getContext('2d');
const ellipse=(cx,cy,rx,ry,n=40,a0=0)=>{const p=[];for(let k=0;k<=n;k++){const a=a0+k/n*Math.PI*2;p.push([cx+Math.cos(a)*rx,cy+Math.sin(a)*ry]);}return p;};

function sticker(){
  const r=482;
  ctx.save(); ctx.beginPath(); ctx.arc(C,C,r,0,Math.PI*2); ctx.clip();
  ctx.fillStyle=PAL.paper; ctx.fillRect(0,0,S,S);
  const g=ctx.getImageData(0,0,S,S), a=g.data, rnd=mulberry(9);
  for(let i=0;i<a.length;i+=4){const n=(rnd()-0.5)*18;a[i]+=n;a[i+1]+=n;a[i+2]+=n;}
  ctx.putImageData(g,0,0);
  const v=ctx.createRadialGradient(C,C,r*0.55,C,C,r);
  v.addColorStop(0,'rgba(60,52,38,0)'); v.addColorStop(1,'rgba(60,52,38,0.16)');
  ctx.fillStyle=v; ctx.fillRect(0,0,S,S);
  ctx.restore();
  ring(ctx,C,C,r-7,{w:12,c:PAL.ink,amp:2.4,n:40});
}

function edge(){
  // the graph edge the owl sits on: two nodes, one certified line between them
  const A=[168,650], B=[856,640];
  ink(ctx,[A,[C,662],B],{w:15,c:PAL.ink,amp:2});
  ink(ctx,[[A[0]+40,A[1]-2],[C,655],[B[0]-40,B[1]-2]],{w:5,c:PAL.teal,amp:1.4,alpha:0.9});
  for(const [x,y] of [A,B]){
    blob(ctx,x,y,52,PAL.tealL,{alpha:0.95,layers:3,wob:0.1});
    blob(ctx,x-10,y-10,26,PAL.teal,{alpha:0.35,layers:2,wob:0.15});
    ring(ctx,x,y,52,{w:10,c:PAL.ink,amp:1.8});
  }
}

function owl(){
  const bx=C, by=478;
  // wings behind the body
  for(const sgn of [-1,1]){
    const w=[[bx+sgn*118,by-70],[bx+sgn*196,by+10],[bx+sgn*206,by+120],[bx+sgn*168,by+178],[bx+sgn*104,by+150]];
    wash(ctx,[...w,w[0]],PAL.amber,{alpha:0.55,spread:6});
    ink(ctx,w,{w:9,c:PAL.ink,amp:2});
    for(let k=0;k<3;k++) ink(ctx,[[bx+sgn*(150+k*14),by+40+k*40],[bx+sgn*(186-k*4),by+72+k*38]],{w:5,c:PAL.ink2,amp:1.2});
  }
  // body
  const body=ellipse(bx,by+18,150,178,44);
  wash(ctx,body,PAL.amberL,{alpha:0.85,spread:7});
  ink(ctx,body,{w:10,c:PAL.ink,amp:2.2,close:true});
  // belly, with rows of feather marks
  const belly=ellipse(bx,by+86,92,98,36);
  wash(ctx,belly,'#FBF4E4',{alpha:0.9,spread:5});
  for(let r=0;r<3;r++) for(let c=-2;c<=2;c++){
    if(r===2&&Math.abs(c)===2) continue;
    const x=bx+c*32+(r%2?16:0), y=by+50+r*36; if(Math.abs(x-bx)>72) continue;
    ink(ctx,[[x-11,y-5],[x,y+5],[x+11,y-5]],{w:5,c:PAL.amber,amp:0.8});
  }
  // ear tufts
  for(const sgn of [-1,1]){
    const t=[[bx+sgn*58,by-150],[bx+sgn*128,by-238],[bx+sgn*132,by-132]];
    wash(ctx,[...t,t[0]],PAL.amber,{alpha:0.75,spread:4});
    ink(ctx,t,{w:9,c:PAL.ink,amp:1.8});
  }
  // facial disc
  for(const sgn of [-1,1]){
    const d=ellipse(bx+sgn*66,by-78,90,86,36);
    wash(ctx,d,'#FBF4E4',{alpha:0.95,spread:4});
  }
  // only the OUTER rim of each disc is inked, so the two meet in a V over the beak
  const arc=(cx,cy,rx,ry,d0,d1,n=28)=>{const p=[];for(let k=0;k<=n;k++){const a=(d0+(d1-d0)*k/n)*Math.PI/180;p.push([cx+Math.cos(a)*rx,cy+Math.sin(a)*ry]);}return p;};
  ink(ctx,arc(bx-66,by-78,90,86,58,302),{w:8,c:PAL.ink,amp:1.4});
  ink(ctx,arc(bx+66,by-78,90,86,238,482),{w:8,c:PAL.ink,amp:1.4});
  // eyes: the one accent is the teal of a certified edge
  for(const sgn of [-1,1]){
    const x=bx+sgn*66, y=by-78;
    blob(ctx,x,y,54,PAL.teal,{alpha:0.9,layers:3,wob:0.06});
    blob(ctx,x,y,40,PAL.tealL,{alpha:0.45,layers:2,wob:0.08});
    ring(ctx,x,y,56,{w:9,c:PAL.ink,amp:1.4});
    ctx.save(); ctx.fillStyle=PAL.ink; ctx.beginPath(); ctx.ellipse(x+sgn*4,y+4,26,28,0,0,Math.PI*2); ctx.fill();
    ctx.fillStyle='#FFFDF6'; ctx.beginPath(); ctx.ellipse(x-10,y-12,10,10,0,0,Math.PI*2); ctx.fill();
    ctx.beginPath(); ctx.ellipse(x+12,y+14,4.5,4.5,0,0,Math.PI*2); ctx.fill(); ctx.restore();
  }
  // beak
  const k=[[bx-22,by-40],[bx+22,by-40],[bx,by+4]];
  wash(ctx,[...k,k[0]],PAL.amber,{alpha:0.95,spread:2});
  ink(ctx,[...k,k[0]],{w:7,c:PAL.ink,amp:1});
  // feet, gripping the edge
  for(const sgn of [-1,1]){
    const fx=bx+sgn*48;
    // three toes curled over the line, claws hooking under it
    for(const d of [-16,0,16]){
      ink(ctx,[[fx+d*0.4,by+168],[fx+d,by+182],[fx+d*1.1,by+196]],{w:12,c:PAL.amber,amp:0.8});
      ink(ctx,[[fx+d*1.1,by+196],[fx+d*1.1-4,by+206]],{w:5,c:PAL.ink,amp:0.5});
    }
  }
}

function wordmark(){
  const txt='Open Ontologies', size=92;
  ctx.save(); ctx.font=`700 ${size}px "CaveatBrush"`; const w=ctx.measureText(txt).width; ctx.restore();
  window.LAYOUT_ERRORS=[];
  const half=w/2, y=826, top=y-size*0.8, r=470;
  // the lettering must sit inside the rim at its widest corner
  for(const yy of [top,y+size*0.22]) if(Math.hypot(half,yy-C)>r-28) window.LAYOUT_ERRORS.push(`wordmark corner at y=${Math.round(yy)} leaves the sticker`);
  hand(ctx,txt,C,y,size,{font:'CaveatBrush',align:'center'});
  ink(ctx,[[C-half*0.8,y+26],[C,y+30],[C+half*0.55,y+24]],{w:10,c:PAL.teal,amp:2});
}

(async()=>{
  const face=new FontFace('CaveatBrush','url(fonts/CaveatBrush.ttf)'); await face.load(); document.fonts.add(face);
  await document.fonts.ready;
  ctx.clearRect(0,0,S,S);
  sticker(); edge(); owl(); wordmark();
  window.READY=true;
})().catch(e=>{window.BOOTERR=String(e.stack||e);});
