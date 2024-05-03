function checkDarkMode() {
  var mode = localStorage.getItem('mode');
  if(mode === 'disabled') {
    document.body.classList.remove("dark-mode");
    document.body.classList.add("bg-light");
  }
}

