// Remove logo from homepage
document.addEventListener('DOMContentLoaded', function() {
  if (window.location.pathname === '/' || window.location.pathname === '/index.html') {
    const logoImg = document.querySelector('nav > img');
    if (logoImg) {
      logoImg.style.display = 'none';
    }
    
    const logoLink = document.querySelector('nav > a > img');
    if (logoLink) {
      logoLink.parentElement.style.display = 'none';
    }
  }
});