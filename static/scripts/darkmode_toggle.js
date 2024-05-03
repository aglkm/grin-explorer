function darkModeFunc() {
  document.body.classList.toggle("dark-mode");
  document.body.classList.toggle("bg-light");

  var mode = localStorage.getItem('mode');
  if(mode === 'disabled')
    localStorage.setItem('mode', 'enabled');
  else
    localStorage.setItem('mode', 'disabled');
}

