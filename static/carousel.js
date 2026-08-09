(()=>{"use strict";
function initCarousel(root){
  const track=root.querySelector(".testimonial-track");
  const slides=track?[...track.children]:[];
  const dotsWrap=root.querySelector(".testimonial-dots");
  if(!track||slides.length<2){root.querySelector(".testimonial-controls")?.style.setProperty("display","none");return;}
  let index=0;
  slides.forEach((_,i)=>{
    const dot=document.createElement("button");
    dot.type="button";
    dot.setAttribute("aria-label",`Go to testimonial ${i+1}`);
    dot.addEventListener("click",()=>go(i));
    dotsWrap.appendChild(dot);
  });
  const dots=[...dotsWrap.children];
  function update(){
    track.scrollTo({left:slides[index].offsetLeft,behavior:"smooth"});
    dots.forEach((d,i)=>d.classList.toggle("active",i===index));
  }
  function go(i){index=(i+slides.length)%slides.length;update();}
  root.querySelector(".carousel-prev")?.addEventListener("click",()=>go(index-1));
  root.querySelector(".carousel-next")?.addEventListener("click",()=>go(index+1));
  let timer=setInterval(()=>go(index+1),6000);
  root.addEventListener("mouseenter",()=>clearInterval(timer));
  root.addEventListener("mouseleave",()=>{timer=setInterval(()=>go(index+1),6000);});
  update();
}
document.querySelectorAll(".testimonial-carousel").forEach(initCarousel);
})();
