(()=>{"use strict";
document.addEventListener("change",e=>{
  const select=e.target.closest("select.admin-autosubmit");
  if(select)select.form?.requestSubmit();
});
document.addEventListener("submit",e=>{
  if(e.target.classList?.contains("admin-confirm-delete")&&!confirm("Are you sure? This cannot be undone.")){
    e.preventDefault();
  }
});
})();
