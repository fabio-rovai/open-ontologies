/* The README's front-page animation, drawn by hand in code.

   Nothing here is data. Every node position, every edge and its warrant, which
   edge is forged, every sentence and every count comes from the scene file that
   `docs/assets/knowledge-graph.py` writes beside the SVG in the same run, so the
   two pictures cannot disagree. This file only decides how things LOOK: paper,
   wobbling ink, watercolour, handwriting (engine.js, the house film engine).

   Two rules carried over from the SVG:
   - Nothing is built up from nothing. A still sampled at t=0 (a thumbnail, a
     link preview, the first frame of the WebP) is the whole graph. A beat
     brightens its layer or inks over a line that is already there.
   - A label that collides or leaves the frame is an error, not a style. All
     geometry is fixed in `layout()` before anything is drawn: every text box is
     checked against every other text box AND against every arrow of the
     pipeline, and `LAYOUT_ERRORS` fails the render. The first version checked
     text against text only, and three labels sat under arrows with the gate
     reporting clean. `?probe=1` plants a collision so the gate can be seen to
     fail (render.js --prove-gate). */
import {W,H,PAL,paper,grainOver,ink,wash,blob,ring,hand as engineHand,seedFrame} from './engine.js';

const ctx=document.getElementById('c').getContext('2d');
const HUE={asserted:PAL.ink2,derived:PAL.teal,lean:PAL.teal,provers:PAL.ink2,
           forged:PAL.red,unasked:PAL.amber};
const WARRANT={asserted:PAL.grey,certified:PAL.teal,rejected:PAL.red,unasked:PAL.amber};
const VERDICT={certificate:PAL.teal,opinion:PAL.ink2};
const FONT={title:'CaveatBrush',body:'PatrickHand',mono:'JetBrainsMono'};
/* The Latin hands have no Chinese glyphs. A string with any CJK in it is set in
   the Chinese hand whatever slot it sits in, and `measure` applies the same rule,
   so the layout gate measures what is actually drawn. */
const CJK=/[\u2e80-\u9fff\u3000-\u303f\uff00-\uffef]/;
let ZH='LXGWWenKai';
const face=(str,font)=>CJK.test(str)?ZH:font;
const hand=(c,str,x,y,size,o={})=>engineHand(c,str,x,y,size,{...o,font:face(str,o.font||'Caveat')});
const LAYOUT_ERRORS=[];

let S,P,R,BEAT,ORDER,NODE_ORDER;
let LABELS,BADGES,ARROWS,FUNNEL,NOTE,FORGED,REFUSED,MU,LEG;
const byLabel=l=>S.nodes.findIndex(n=>n.label===l);

/* ---------- where things go ---------- */
function measure(str,size,font){
  ctx.save();ctx.font=`700 ${size}px "${face(str,font)}"`;const w=ctx.measureText(str).width;ctx.restore();
  return w;
}
function place(){
  // The ontology keeps its aspect: stretching it would bend the perspective the
  // 3D layout exists to show. It is fitted into the right-hand region.
  const [gx0,gy0,gx1,gy1]=S.box.graph;
  const RX0=800,RY0=300,RX1=1870,RY1=800;
  const gs=Math.min((RX1-RX0)/(gx1-gx0),(RY1-RY0)/(gy1-gy0));
  const ox=RX0+((RX1-RX0)-(gx1-gx0)*gs)/2, oy=RY0+((RY1-RY0)-(gy1-gy0)*gs)/2;
  // The pipeline is PLACED by the generator; here it is scaled, and the one
  // node to the right of a judge is pushed clear of that judge's name.
  P=S.nodes.map(n=>n.kind==='class'
    ? {x:ox+(n.x-gx0)*gs, y:oy+(n.y-gy0)*gs}
    : {x:130+(n.x-86)*1.42, y:305+(n.y-168)*1.38});
  R=S.nodes.map(n=>n.kind==='lean'?26:n.kind==='file'?32:n.kind==='program'?15
    :(2.2+Math.min(4.6,n.deg*0.30))*(0.62+0.62*n.depth)*gs*1.2);
  S.nodes.forEach((n,i)=>{ if(n.kind!=='program') return;
    for(const [a,b,] of S.edges) if(b===i&&S.nodes[a].kind==='program'&&P[a].y===P[i].y)
      P[i].x=Math.max(P[i].x,P[a].x+R[a]+18+measure(S.nodes[a].label,26,FONT.body)+70+R[i]); });
  const cls=S.nodes.map((n,i)=>i).filter(i=>S.nodes[i].kind==='class');
  S.panel=[Math.min(...cls.map(i=>P[i].x))-34,Math.min(...cls.map(i=>P[i].y))-30,
           Math.max(...cls.map(i=>P[i].x))+34,Math.max(...cls.map(i=>P[i].y))+30];
  BEAT=Object.fromEntries(S.beats.map(b=>[b.key,b]));
  const dep=e=>S.nodes[S.edges[e][0]].depth+S.nodes[S.edges[e][1]].depth;
  ORDER=S.edges.map((e,k)=>k).sort((a,b)=>dep(a)-dep(b));
  NODE_ORDER=S.nodes.map((n,i)=>i).sort((a,b)=>S.nodes[a].depth-S.nodes[b].depth);
}

/* ---------- the layout gate ---------- */
const boxes=[], segs=[];
const hitsBox=(b,o)=>!(b[2]<o[0]||b[0]>o[2]||b[3]<o[1]||b[1]>o[3]);
function segHits(s,b){
  const [a,c]=s, n=Math.ceil(Math.hypot(c[0]-a[0],c[1]-a[1])/3);
  for(let k=0;k<=n;k++){ const x=a[0]+(c[0]-a[0])*k/n, y=a[1]+(c[1]-a[1])*k/n;
    if(x>b[0]&&x<b[2]&&y>b[1]&&y<b[3]) return true; }
  return false;
}
function free(b,{lines=true}={}){
  return b[0]>=16&&b[1]>=8&&b[2]<=W-16&&b[3]<=H-8
    && boxes.every(o=>!hitsBox(b,o.b)) && (!lines||segs.every(s=>!segHits(s.p,b)));
}
function claim(b,where,{lines=true}={}){
  if(b[0]<16||b[1]<8||b[2]>W-16||b[3]>H-8) LAYOUT_ERRORS.push(`${where} leaves the frame`);
  for(const o of boxes) if(hitsBox(b,o.b)) LAYOUT_ERRORS.push(`${where} overlaps ${o.where}`);
  if(lines) for(const s of segs) if(segHits(s.p,b)) LAYOUT_ERRORS.push(`${where} lies under ${s.where}`);
  boxes.push({b,where});
}
/* the first candidate that is free, else the last one claimed anyway, which
   the gate then reports */
function why(b,{lines=true}={}){
  const r=[]; if(b[0]<16||b[1]<8||b[2]>W-16||b[3]>H-8) r.push('frame');
  for(const o of boxes) if(hitsBox(b,o.b)) r.push(o.where);
  if(lines) for(const s of segs) if(segHits(s.p,b)) r.push(s.where);
  return r;
}
const TRACE=[];
function pick(cands,where,opts){
  TRACE.push([where,cands.map(b=>why(b,opts))]);
  const c=cands.find(b=>free(b,opts))||cands[cands.length-1];
  claim(c,where,opts); return c;
}
const textBox=(x,y,str,size,font,align='left')=>{
  const w=measure(str,size,font), x0=align==='center'?x-w/2:align==='right'?x-w:x;
  return [x0-4,y-size*0.82,x0+w+4,y+size*0.26];
};

function layout(){
  boxes.length=0; segs.length=0; LAYOUT_ERRORS.length=0;
  claim(textBox(80,100,S.headline,64,FONT.title),'headline');
  claim(textBox(80,152,S.sub,19,FONT.mono),'sub-heading');
  const cap=S.beats.reduce((a,b)=>measure(b.text,38,FONT.body)>measure(a.text,38,FONT.body)?b:a);
  claim(textBox(80,214,cap.text,38,FONT.body),'caption');
  const heads=[byLabel('ies-core.ttl'),byLabel('certificate'),S.nodes.findIndex(n=>n.kind==='lean')];
  S.cols.forEach((c,m)=>claim(textBox(P[heads[m]].x,272,c.toUpperCase(),15,FONT.mono,'center'),`column ${c}`));
  claim(textBox(S.panel[0],272,S.graphhead,16,FONT.mono),'graph heading');
  claim(S.panel,'ontology panel',{lines:false});

  // arrows of the pipeline, fixed before any label is placed
  ARROWS=[];
  for(const [i,j,w] of S.edges){
    if(S.nodes[i].kind==='class'||S.nodes[j].kind==='class') continue;
    const a=[P[i].x,P[i].y], b=[P[j].x,P[j].y], L=Math.hypot(b[0]-a[0],b[1]-a[1]);
    const ux=(b[0]-a[0])/L, uy=(b[1]-a[1])/L, rj=R[j]+10;
    // a judge feeding the judge beside it: the arrow leaves after the first one's name
    const ri=S.nodes[i].kind==='program'&&S.nodes[j].kind==='program'
      ? R[i]+18+measure(S.nodes[i].label,26,FONT.body)+14 : R[i]+8;
    const beat=S.nodes[j].kind==='file'?'asserted':(w==='certified'&&S.nodes[i].label==='certificate')?'lean':'provers';
    const A=[a[0]+ux*ri,a[1]+uy*ri], B=[b[0]-ux*rj,b[1]-uy*rj];
    ARROWS.push({A,B,w,beat}); segs.push({p:[A,B],where:`the arrow ${S.nodes[i].label} to ${S.nodes[j].label}`});
  }
  // the four provers' answers funnel back to the certificate: a disagreement stops the line
  const pv=['Vampire','E','Z3','Mace4'].map(byLabel), ci=byLabel('certificate');
  const fx=Math.min(...pv.map(i=>P[i].x))-58, fy=pv.reduce((s,i)=>s+P[i].y,0)/pv.length;
  FUNNEL={stubs:pv.map(i=>[[P[i].x-R[i]-6,P[i].y],[fx,P[i].y]]),
          spine:[[fx,Math.min(...pv.map(i=>P[i].y))],[fx,Math.max(...pv.map(i=>P[i].y))]],
          back:[[fx,fy],[P[ci].x+14,P[ci].y+R[ci]+12]]};
  FUNNEL.stubs.forEach(s=>segs.push({p:s,where:'the disagreement funnel'}));
  segs.push({p:FUNNEL.spine,where:'the disagreement funnel'},{p:FUNNEL.back,where:'the disagreement arrow'});

  // names of the pipeline: sheets are named underneath, judges to the right
  LABELS=[];
  S.nodes.forEach((n,i)=>{ if(n.kind==='class') return;
    const size=n.kind==='lean'?30:26;
    const cands=n.kind==='file'
      ? [[P[i].x,P[i].y+R[i]+34,'center'],[P[i].x,P[i].y-R[i]-12,'center'],
         [P[i].x+8,P[i].y+R[i]+34,'right'],[P[i].x+R[i]+14,P[i].y+9,'left']]
      : [[P[i].x+R[i]+18,P[i].y+9,'left'],[P[i].x,P[i].y-R[i]-16,'center'],[P[i].x,P[i].y+R[i]+32,'center']];
    const bs=cands.map(([x,y,al])=>textBox(x,y,n.label,size,FONT.body,al));
    const b=pick(bs,`the name ${n.label}`);
    const [x,y,al]=cands[bs.indexOf(b)];
    LABELS.push({i,x,y,al,size,b});
  });
  // each judge's verdict tag: after its name, else above, else below the node
  BADGES=S.judges.map(j=>{
    const i=j.node, lab=LABELS.find(l=>l.i===i), w=measure(j.verdict,21,FONT.body)+26, h=32;
    const cands=[[lab.b[2]+12,P[i].y-h/2],[P[i].x-w/2,P[i].y-R[i]-h-10],[P[i].x-w/2,P[i].y+R[i]+10]]
      .map(([x,y])=>[x,y,x+w,y+h]);
    const b=pick(cands,`the verdict ${j.verdict} of ${S.nodes[i].label}`);
    return {...j,x:b[0],y:b[1],w,h};
  });
  const note=S.labels.disagree, m4=byLabel('Mace4');
  NOTE=pick([textBox(fx-24,P[m4].y+52,note,21,FONT.body),textBox(fx-150,P[m4].y+52,note,21,FONT.body)],'the disagreement note');

  // the busy classes, named only where the name lands clear (the SVG's rule)
  S.nodes.forEach((n,i)=>{ if(n.kind!=='class'||!n.labelled) return;
    const c=[[P[i].x+R[i]+8,P[i].y+8],[P[i].x-R[i]-8-measure(n.label,23,FONT.body),P[i].y+8],
             [P[i].x-measure(n.label,23,FONT.body)/2,P[i].y-R[i]-8]]
      .map(([x,y])=>textBox(x,y,n.label,23,FONT.body));
    boxes.splice(boxes.findIndex(o=>o.where==='ontology panel'),1);
    const b=c.find(b=>free(b,{lines:false}));
    boxes.push({b:S.panel,where:'ontology panel'});
    if(b){ boxes.splice(boxes.length-1,0,{b,where:`the class ${n.label}`}); LABELS.push({i,x:b[0]+4,y:P[i].y+8,al:'left',size:23,cls:true,b}); }
  });
  // the words of beats 5 and 6 go INSIDE the panel, clear of the class names
  const inPanel=b=>b[0]>=S.panel[0]&&b[2]<=S.panel[2]&&b[1]>=S.panel[1]&&b[3]<=S.panel[3];
  const panelOff=()=>boxes.filter(o=>o.where!=='ontology panel');
  const placeIn=(cands,where)=>{
    const ok=cands.find(b=>inPanel(b)&&panelOff().every(o=>!hitsBox(b,o.b)))||cands[cands.length-1];
    if(!inPanel(ok)) LAYOUT_ERRORS.push(`${where} leaves the ontology panel`);
    for(const o of panelOff()) if(hitsBox(ok,o.b)) LAYOUT_ERRORS.push(`${where} overlaps ${o.where}`);
    boxes.push({b:ok,where}); return ok;
  };
  if(S.forged!==null){
    const [i,j]=S.edges[S.forged], mx=(P[i].x+P[j].x)/2, my=(P[i].y+P[j].y)/2;
    const ry=Math.abs(P[i].y-P[j].y)/2+22, rx=Math.abs(P[i].x-P[j].x)/2+26;
    const f=S.labels.forged;
    const b=placeIn([textBox(mx+rx-10,my-ry-10,f,28,FONT.body),textBox(mx-rx+10-measure(f,28,FONT.body),my-ry-10,f,28,FONT.body),
                     textBox(mx-measure(f,28,FONT.body)/2,my+ry+34,f,28,FONT.body)],'the forged-line note');
    FORGED={mx,my,rx,ry,x:b[0]+4,y:b[3]-28*0.26};
    const li=S.nodes.findIndex(n=>n.kind==='lean'), lab=LABELS.find(l=>l.i===li);
    const rb=pick([textBox(lab.b[0]+4,P[li].y+48,S.labels.refused,24,FONT.body)],'the refusal');
    REFUSED={x:rb[0]+4,y:P[li].y+48,li};
  }
  if(S.mu){
    const a=P[S.mu.s], b=P[S.mu.o], mx=(a.x+b.x)/2, my=(a.y+b.y)/2, t=S.mu.badge, w=measure(t,24,FONT.body);
    const glyph=placeIn([[mx-32,my-40,mx+32,my+26]],'the mark of mu');
    const bb=placeIn([-78,70,-110,110,-150].map(dy=>textBox(Math.min(Math.max(mx-w/2,S.panel[0]+14),S.panel[2]-w-14),my+dy,t,24,FONT.body)),
                     'the unasked badge');
    MU={mx,my,x:bb[0]+4,y:bb[3]-24*0.26,w,glyph};
  }
  legendLayout();
  if(new URLSearchParams(location.search).get('probe')){
    const l=LABELS.find(l=>S.nodes[l.i].label==='Vampire'); claim(l.b.slice(),'a planted collision');
  }
}

/* ---------- time ---------- */
const clamp=v=>Math.max(0,Math.min(1,v));
/* 0 outside [t0,t1], 1 inside, with short ramps. Consecutive beats share an
   edge, so two captions are never drawn through each other. */
function on(t,t0,t1,ri=0.35,ro=0.35){ return clamp((t-t0)/ri)*clamp((t1-t)/ro); }
function beatAt(t){ let k=0; S.beats.forEach((b,i)=>{ if(t>=b.t0) k=i; }); return k; }

/* ---------- props ---------- */
function sheet(x,y,{seal=false}={}){
  const w=50,h=64,f=14;
  const p=[[x-w/2,y-h/2],[x+w/2-f,y-h/2],[x+w/2,y-h/2+f],[x+w/2,y+h/2],[x-w/2,y+h/2],[x-w/2,y-h/2]];
  wash(ctx,p,'#FBF7EC',{alpha:0.9,spread:3});
  ink(ctx,p,{w:4,c:PAL.ink,amp:1.6});
  ink(ctx,[[x+w/2-f,y-h/2],[x+w/2-f,y-h/2+f],[x+w/2,y-h/2+f]],{w:3,c:PAL.ink,amp:1});
  for(let k=0;k<3;k++) ink(ctx,[[x-w/2+9,y-h/2+20+k*11],[x+(k%2?4:w/2-10),y-h/2+20+k*11]],{w:2.4,c:PAL.grey,amp:1});
  if(seal){ blob(ctx,x+12,y+18,11,PAL.amberL,{alpha:0.9,layers:2,wob:0.1});
    ring(ctx,x+12,y+18,11,{w:3,c:PAL.amber,amp:1});
    ink(ctx,[[x+7,y+28],[x+4,y+42]],{w:3,c:PAL.amber,amp:1});
    ink(ctx,[[x+17,y+28],[x+20,y+42]],{w:3,c:PAL.amber,amp:1}); }
}
function judge(i,kind){
  const {x,y}=P[i], r=R[i];
  blob(ctx,x,y,r,kind==='opinion'?PAL.greyL:PAL.tealL,{alpha:0.85,layers:2,wob:0.12});
  ring(ctx,x,y,r,{w:S.nodes[i].kind==='lean'?5:3.5,c:PAL.ink,amp:1.4});
}
function arrow(a,b,{c=PAL.ink2,w=3,alpha=1}={}){
  ink(ctx,[a,b],{w,c,amp:1.4,alpha,passes:1});
  const ang=Math.atan2(b[1]-a[1],b[0]-a[0]), L=13;
  ink(ctx,[[b[0]-L*Math.cos(ang-0.45),b[1]-L*Math.sin(ang-0.45)],b,
           [b[0]-L*Math.cos(ang+0.45),b[1]-L*Math.sin(ang+0.45)]],{w,c,amp:0.8,alpha,passes:1});
}
function dashed(a,b,{c,w=3,alpha=1,dash=14,gap=10,march=0}){
  const L=Math.hypot(b[0]-a[0],b[1]-a[1]), ux=(b[0]-a[0])/L, uy=(b[1]-a[1])/L;
  for(let s=-(march%(dash+gap));s<L;s+=dash+gap){
    const s0=Math.max(0,s), s1=Math.min(L,s+dash); if(s1<=s0) continue;
    ink(ctx,[[a[0]+ux*s0,a[1]+uy*s0],[a[0]+ux*s1,a[1]+uy*s1]],{w,c,amp:0.8,alpha,passes:1});
  }
}
/* the marker circle a teacher draws round the thing being talked about */
function circleAround(x,y,rx,ry,c,prog,alpha=1){
  if(prog<=0) return;
  const pts=[]; for(let k=0;k<=34;k++){ const a=-2.2+k/34*Math.PI*2.15, g=1+0.04*k/34;
    pts.push([x+Math.cos(a)*rx*g,y+Math.sin(a)*ry*g]); }
  ink(ctx,pts,{w:3.5,c,amp:1.6,alpha,draw:prog,passes:1});
}
function tag(b,alpha,lit){
  const col=VERDICT[b.kind];
  const p=[[b.x,b.y],[b.x+b.w,b.y],[b.x+b.w,b.y+b.h],[b.x,b.y+b.h],[b.x,b.y]];
  wash(ctx,p,b.kind==='opinion'?'#E9E4D8':'#DCEBE8',{alpha:0.95*alpha,spread:2});
  ink(ctx,p,{w:lit?3.2:2.4,c:col,amp:1.2,alpha});
  hand(ctx,b.verdict,b.x+b.w/2,b.y+b.h-9,21,{font:FONT.body,c:col,align:'center',alpha});
}

/* ---------- the legend ---------- */
function legendLayout(){
  const x0=70,y0=832,right=1000; LEG={x0,y0,right};
  const L=S.legend;
  claim(textBox(x0+30,y0+46,L.head,20,FONT.mono),'legend heading');
  claim(textBox(x0+470,y0+46,L.file,17,FONT.mono),'legend file');
  L.rows.forEach((r,m)=>{ const y=y0+92+m*36;
    claim(textBox(x0+84,y,r.label,18,FONT.mono),`legend label ${r.label}`);
    const b=textBox(x0+300,y,r.means,24,FONT.body); claim(b,`legend row ${r.label}`);
    if(b[2]>right-30) LAYOUT_ERRORS.push(`legend row ${r.label} runs into the second column`); });
  claim(textBox(right+30,y0+46,S.worth.head,20,FONT.mono),'worth heading');
  S.worth.rows.forEach((r,m)=>{ const y=y0+84+m*56;
    claim(textBox(right+62,y+4,r.word,26,FONT.body),`worth word ${r.word}`);
    for(const [s,sz,dy,k] of [[r.l1,22,-8,1],[r.l2,19,17,2]]){
      const b=textBox(right+190,y+dy,s,sz,FONT.body); claim(b,`worth ${r.word} line ${k}`);
      if(b[2]>W-x0-20) LAYOUT_ERRORS.push(`worth ${r.word} line ${k} runs past the card`); } });
}
function legend(t){
  const {x0,y0,right}=LEG, L=S.legend;
  const card=[[x0,y0],[W-x0,y0],[W-x0,H-24],[x0,H-24],[x0,y0]];
  wash(ctx,card,'#FBF7EC',{alpha:0.55,spread:4});
  ink(ctx,card,{w:3,c:PAL.ink2,amp:1.6});
  hand(ctx,L.head,x0+30,y0+46,20,{font:FONT.mono,c:PAL.teal});
  hand(ctx,L.file,x0+470,y0+46,17,{font:FONT.mono,c:PAL.textDim});
  ink(ctx,[[x0+30,y0+62],[right-40,y0+62]],{w:2,c:PAL.greyL,amp:1});
  const beatOf={asserted:'asserted',certified:'lean',rejected:'forged',unasked:'unasked'};
  L.rows.forEach((r,m)=>{ const y=y0+92+m*36, c=WARRANT[r.key];
    const b=BEAT[beatOf[r.key]], lit=b?on(t,b.t0,b.t1):0;
    ink(ctx,[[x0+30,y-8],[x0+70,y-8]],{w:5+lit*3,c,amp:1.2});
    hand(ctx,r.label,x0+84,y,18,{font:FONT.mono,c});
    hand(ctx,String(r.count),x0+280,y,19,{font:FONT.mono,c:PAL.ink,align:'right'});
    hand(ctx,r.means,x0+300,y,24,{font:FONT.body,c:PAL.ink2});
  });
  ink(ctx,[[right,y0+26],[right,H-50]],{w:2,c:PAL.greyL,amp:1.2});
  hand(ctx,S.worth.head,right+30,y0+46,20,{font:FONT.mono,c:PAL.ink2});
  const wc={certificate:PAL.teal,opinion:PAL.ink2,unasked:PAL.amber};
  S.worth.rows.forEach((r,m)=>{ const y=y0+84+m*56, c=wc[r.key];
    blob(ctx,right+42,y-2,9,c,{alpha:0.9,layers:2,wob:0.1});
    hand(ctx,r.word,right+62,y+4,26,{font:FONT.body,c});
    hand(ctx,r.l1,right+190,y-8,22,{font:FONT.body,c:PAL.ink});
    hand(ctx,r.l2,right+190,y+17,19,{font:FONT.body,c:PAL.textDim});
  });
}

/* ---------- one frame ---------- */
function drawAt(t){
  seedFrame(0);
  ctx.setTransform(1,0,0,1,0,0);
  paper(ctx);
  const k=beatAt(t), cur=S.beats[k];
  const A=BEAT.asserted, D=BEAT.derived, L=BEAT.lean, F=BEAT.forged, U=BEAT.unasked;

  // headings, and the running step; the first is on from t=0 so a still has one
  hand(ctx,S.headline,80,100,64,{font:FONT.title});
  ink(ctx,[[80,116],[80+measure(S.headline,64,FONT.title)*0.62,120]],{w:6,c:PAL.teal,amp:2});
  hand(ctx,S.sub,80,152,19,{font:FONT.mono,c:PAL.textDim});
  const cin=k===0?1:clamp((t-cur.t0)/0.3), cout=k===S.beats.length-1?1:clamp((cur.t1-t)/0.3);
  hand(ctx,cur.text,80,214,38,{font:FONT.body,c:HUE[cur.key],alpha:Math.max(0.001,Math.min(cin,cout))});
  const heads=[byLabel('ies-core.ttl'),byLabel('certificate'),S.nodes.findIndex(n=>n.kind==='lean')];
  S.cols.forEach((c,m)=>hand(ctx,c.toUpperCase(),P[heads[m]].x,272,15,{font:FONT.mono,c:PAL.grey,align:'center'}));
  hand(ctx,S.graphhead,S.panel[0],272,16,{font:FONT.mono,c:PAL.grey});

  // the ontology's panel
  const [px0,py0,px1,py1]=S.panel;
  const pan=[[px0,py0],[px1,py0],[px1,py1],[px0,py1],[px0,py0]];
  wash(ctx,pan,'#EFE8D6',{alpha:0.35,spread:6});
  ink(ctx,pan,{w:2.4,c:PAL.greyL,amp:2});

  const litA=on(t,A.t0,A.t1), derive=clamp((t-D.t0)/(D.t1-D.t0-0.4)),
        litD=on(t,D.t0,L.t1), litL=on(t,L.t0,L.t1), sweep=clamp((t-L.t0)/(L.t1-L.t0-0.6)),
        litF=on(t,F.t0,F.t1), litU=U?on(t,U.t0,U.t1):0;

  // edges of the ontology, far to near
  for(const e of ORDER){
    const [i,j,w]=S.edges[e];
    if(S.nodes[i].kind!=='class'||S.nodes[j].kind!=='class'||e===S.forged) continue;
    const d=(S.nodes[i].depth+S.nodes[j].depth)/2, a=[P[i].x,P[i].y], b=[P[j].x,P[j].y];
    if(w==='asserted'){
      ink(ctx,[a,b],{w:1.3+litA*0.6,c:litA>0.01?PAL.ink2:PAL.grey,amp:1,passes:1,alpha:(0.18+0.34*d)*(0.6+0.4*litA)});
    } else {
      ink(ctx,[a,b],{w:1.6,c:PAL.teal,amp:1,passes:1,alpha:0.14+0.3*d});
      // the engine derives from left to right, then Lean's sweep inks each one it checks
      const fx=((a[0]+b[0])/2-px0)/(px1-px0), drawn=clamp((derive-fx*0.7)/0.3);
      if(litD>0&&drawn>0) ink(ctx,[a,b],{w:2.4,c:PAL.teal,amp:1,passes:1,alpha:litD*(0.45+0.45*d),draw:drawn});
      if(litL>0&&sweep>fx) ink(ctx,[a,b],{w:3.2,c:PAL.teal,amp:1,passes:1,alpha:litL*0.5});
    }
  }
  // the forged line: among the rest, faint red until the checker names it
  if(S.forged!==null){ const [i,j]=S.edges[S.forged];
    dashed([P[i].x,P[i].y],[P[j].x,P[j].y],{c:PAL.red,w:3+litF*3,alpha:0.45+0.55*litF,march:t*22}); }
  // the unasked question
  if(S.mu){ const a=P[S.mu.s], b=P[S.mu.o];
    dashed([a.x,a.y],[b.x,b.y],{c:PAL.amber,w:3+litU*2,alpha:0.35+0.6*litU,dash:6,gap:9}); }
  // nodes, far to near
  for(const i of NODE_ORDER){ const n=S.nodes[i]; if(n.kind!=='class') continue;
    const r=R[i], a=0.35+0.55*n.depth;
    if(r>=6){ blob(ctx,P[i].x,P[i].y,r,PAL.tealL,{alpha:a,layers:2,wob:0.12});
      ring(ctx,P[i].x,P[i].y,r,{w:2.2,c:PAL.ink,amp:0.9,n:16}); }
    else blob(ctx,P[i].x,P[i].y,r*0.9,PAL.ink2,{alpha:a*0.8,layers:1,wob:0.1});
  }

  // the pipeline
  for(const a of ARROWS){ const lit=on(t,BEAT[a.beat].t0,BEAT[a.beat].t1);
    arrow(a.A,a.B,{c:a.w==='certified'?PAL.teal:PAL.ink2,w:2.6+lit*1.4,alpha:0.5+0.5*lit}); }
  const [dt0,dt1]=S.labels.disagree_t, ld=on(t,dt0,dt1,0.5,0.3), fa=0.22+0.63*ld;
  for(const s of FUNNEL.stubs) ink(ctx,s,{w:2,c:PAL.ink2,amp:1,alpha:fa,passes:1});
  ink(ctx,FUNNEL.spine,{w:2,c:PAL.ink2,amp:1,alpha:fa,passes:1});
  arrow(FUNNEL.back[0],FUNNEL.back[1],{c:PAL.ink2,w:2.4,alpha:fa});
  hand(ctx,S.labels.disagree,NOTE[0]+4,NOTE[3]-21*0.26,21,{font:FONT.body,c:PAL.ink2,alpha:0.3+0.7*ld});

  S.nodes.forEach((n,i)=>{
    if(n.kind==='file') sheet(P[i].x,P[i].y,{seal:n.label==='certificate'});
    else if(n.kind!=='class'){ const jd=S.judges.find(j=>j.node===i); judge(i,jd?jd.kind:'opinion'); }
  });
  // verdict tags: resting faint, lit on their own beat, with a pop ring
  for(const b of BADGES){ const lit=on(t,b.t0,b.t1);
    tag(b,0.4+0.6*lit,lit>0.5);
    const pop=clamp((t-b.t0)/0.6);
    if(t>=b.t0&&pop<1) ring(ctx,P[b.node].x,P[b.node].y,R[b.node]*(1+1.2*pop),{w:3*(1-pop)+0.5,c:VERDICT[b.kind],amp:1});
  }
  for(const l of LABELS){ const n=S.nodes[l.i];
    if(l.cls){ wash(ctx,[[l.b[0],l.b[1]],[l.b[2],l.b[1]],[l.b[2],l.b[3]],[l.b[0],l.b[3]]],PAL.paper,{alpha:0.85,spread:2});
      hand(ctx,n.label,l.x,l.y,l.size,{font:FONT.body,c:PAL.ink2}); }
    else hand(ctx,n.label,l.x,l.y,l.size,{font:FONT.body,c:n.kind==='lean'?PAL.teal:PAL.ink,align:l.al});
  }
  // who is acting now: a marker circle round them for the whole step
  for(const sp of S.spot){ const b=S.beats[sp.beat], a=on(t,b.t0,b.t1,0.2,0.3);
    if(a<=0) continue; const prog=clamp((t-b.t0)/0.6);
    for(const i of sp.nodes){ const f=S.nodes[i].kind==='file';
      circleAround(P[i].x,P[i].y,f?38:R[i]+9,f?44:R[i]+8,HUE[b.key],prog,a); } }

  // beat 5: the forged line is named, and the checker refuses the certificate carrying it
  if(litF>0&&FORGED){ const {mx,my,rx,ry}=FORGED;
    circleAround(mx,my,rx,ry,PAL.red,clamp((t-F.t0)/0.7),litF);
    hand(ctx,S.labels.forged,FORGED.x,FORGED.y,28,{font:FONT.body,c:PAL.red,alpha:litF});
    const li=REFUSED.li, cx=P[li].x, cy=P[li].y, st=clamp((t-F.t0-1.2)/0.4);
    if(st>0){ const s=27*(1.25-0.25*st);
      ink(ctx,[[cx-s,cy-s],[cx+s,cy+s]],{w:7,c:PAL.red,amp:1.5,alpha:litF*st});
      ink(ctx,[[cx+s,cy-s],[cx-s,cy+s]],{w:7,c:PAL.red,amp:1.5,alpha:litF*st});
      hand(ctx,S.labels.refused,REFUSED.x,REFUSED.y,24,{font:FONT.body,c:PAL.red,alpha:litF*st}); }
  }
  // beat 6: a question the file cannot be asked comes back unasked
  if(litU>0&&MU){ const st=clamp((t-U.t0-1.4)/0.5), g=MU.glyph;
    blob(ctx,MU.mx,MU.my-6,34,PAL.paper,{alpha:0.9*litU*st,layers:2,wob:0.08});
    ctx.save();ctx.globalAlpha=litU*st;ctx.fillStyle=PAL.amber;
    ctx.font='700 50px "Hiragino Mincho ProN","Songti SC","Noto Serif CJK SC",serif';
    ctx.textAlign='center';ctx.fillText('無',MU.mx,g[3]-8);ctx.restore();
    wash(ctx,[[MU.x-10,MU.y-26],[MU.x+MU.w+10,MU.y-26],[MU.x+MU.w+10,MU.y+10],[MU.x-10,MU.y+10]],'#F6E7CF',{alpha:0.95*litU,spread:2});
    hand(ctx,S.mu.badge,MU.x,MU.y,24,{font:FONT.body,c:PAL.amber,alpha:litU});
  }

  legend(t);
  grainOver(ctx,t);
}

/* ---------- boot ---------- */
(async()=>{
  const q=new URLSearchParams(location.search);
  S=await (await fetch(q.get('scene')||'../knowledge-graph.scene.json')).json();
  const fonts=[['CaveatBrush','CaveatBrush.ttf'],['PatrickHand','PatrickHand.ttf'],['JetBrainsMono','JetBrainsMono.ttf']];
  if(S.lang==='zh') fonts.push([ZH,'LXGWWenKai.ttf']);
  for(const [n,f] of fonts){ const ff=new FontFace(n,`url(fonts/${f})`); await ff.load(); document.fonts.add(ff); }
  await document.fonts.ready;
  place(); layout();
  window.TRACE=TRACE; window.CYCLE=S.cycle; window.LAYOUT_ERRORS=LAYOUT_ERRORS; window.drawAt=drawAt;
  drawAt(Number(q.get('t')||0));
  window.READY=true;
})().catch(e=>{window.BOOTERR=String(e.stack||e);});
