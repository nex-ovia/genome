const qs = s => document.querySelector(s);
const el = (tag, cls, html) => { const e = document.createElement(tag); if (cls) e.className = cls; if (html != null) e.innerHTML = html; return e; };

async function init() {
  let data;
  try { data = await (await fetch('content.json')).json(); }
  catch (err) { console.error('content load failed', err); return; }
  render(data);
  behaviors();
  loadReleases(data.repo, data.links.releases);
}

function render(data) {
  document.title = `${data.site.name} — ${data.site.tagline}`;

  // Nav
  const nav = qs('#nav-links');
  data.nav.forEach(item => {
    const li = el('li'); const a = el('a', null, item.label); a.href = item.href;
    li.appendChild(a); nav.appendChild(li);
  });
  const sLi = el('li', 'highlight'); const sA = el('a', null, 'Sponsor');
  sA.href = data.links.sponsor; sA.rel = 'noopener noreferrer'; sA.target = '_blank';
  sLi.appendChild(sA); nav.appendChild(sLi);

  // Hero
  qs('#hero-eyebrow').textContent = data.hero.eyebrow;
  qs('#hero-title').textContent = data.site.tagline;
  qs('#hero-sub').textContent = data.site.description;
  const actions = qs('#hero-actions');
  const p = el('a', 'btn btn-primary', data.hero.primary.label + ' &rarr;'); p.href = data.hero.primary.href;
  const s = el('a', 'btn btn-secondary', data.hero.secondary.label + ' &rarr;'); s.href = data.hero.secondary.href; s.rel = 'noopener noreferrer'; s.target = '_blank';
  actions.append(p, s);

  // Value
  const vg = qs('#value-grid');
  data.value.forEach(v => vg.appendChild(el('div', 'card card-split reveal', `
    <h3 class="card-title">${v.title}</h3>
    <p><span class="lead">Problem</span>${v.problem}</p>
    <p><span class="lead good">genome</span>${v.solution}</p>`)));

  // How
  const hg = qs('#how-grid');
  data.how.forEach((h, i) => hg.appendChild(el('div', 'card how-step reveal', `
    <span class="num">0${i + 1}</span>
    <code>genome ${h.step}</code>
    <p class="card-body" style="margin-top:0.75rem;">${h.desc}</p>`)));

  // Use cases
  const ug = qs('#use-grid');
  data.use.forEach(u => ug.appendChild(el('div', 'card reveal', `
    <h3 class="card-title">${u.title}</h3>
    <p class="card-body">${u.desc}</p>`)));

  // Install
  qs('#install-unix').textContent = data.install.unix;
  qs('#install-windows').textContent = data.install.windows;
  qs('#install-note').textContent = data.install.note;

  // Sponsor
  qs('#sponsor-label').textContent = data.sponsor.label;
  qs('#sponsor-title').textContent = data.sponsor.title;
  qs('#sponsor-body').textContent = data.sponsor.body;
  const sc = qs('#sponsor-cta'); sc.textContent = data.sponsor.cta; sc.href = data.links.sponsor;

  // Footer
  const fl = qs('#footer-links');
  data.footer.links.forEach(l => { const li = el('li'); const a = el('a', null, l.label); a.href = l.href; li.appendChild(a); fl.appendChild(li); });
  qs('#footer-copy').textContent = `genome is a nex-ovia tool · source-available under BSL 1.1 · built on the Nexovia Standard`;
}

const PLATFORMS = [
  { match: 'x86_64-unknown-linux-musl', os: 'Linux', arch: 'x86_64 · static' },
  { match: 'aarch64-apple-darwin',      os: 'macOS', arch: 'Apple Silicon' },
  { match: 'x86_64-apple-darwin',       os: 'macOS', arch: 'Intel' },
  { match: 'x86_64-pc-windows-msvc',    os: 'Windows', arch: 'x86_64' },
];

async function loadReleases(repo, releasesUrl) {
  const head = qs('#release-head');
  const list = qs('#release-list');
  try {
    const res = await fetch(`https://api.github.com/repos/${repo}/releases?per_page=1`);
    if (!res.ok) throw new Error('api ' + res.status);
    const rel = (await res.json())[0];
    if (!rel) throw new Error('no releases');
    head.innerHTML = `Latest: <span class="ver">${rel.tag_name}</span>${rel.prerelease ? '<span class="pre">pre-release</span>' : ''}`;
    const bins = rel.assets.filter(a => !a.name.endsWith('.sha256'));
    PLATFORMS.forEach(pf => {
      const asset = bins.find(a => a.name.includes(pf.match));
      if (!asset) return;
      const mb = (asset.size / 1048576).toFixed(1);
      list.appendChild(el('div', 'release-row', `
        <div><span class="os">${pf.os}</span><span class="arch">${pf.arch}</span></div>
        <div style="display:flex;align-items:center;gap:1.25rem;"><span class="meta">${mb} MB</span><a href="${asset.browser_download_url}">Download &darr;</a></div>`));
    });
  } catch (err) {
    head.textContent = 'Latest release';
    list.innerHTML = `<div class="release-row"><span class="meta">See all downloads on <a href="${releasesUrl}">GitHub Releases &rarr;</a></span></div>`;
  }
}

function behaviors() {
  const nav = qs('#nav');
  const onScroll = () => nav.classList.toggle('scrolled', window.scrollY > 20);
  window.addEventListener('scroll', onScroll); onScroll();

  const io = new IntersectionObserver(entries => {
    entries.forEach(e => { if (e.isIntersecting) { e.target.classList.add('visible'); io.unobserve(e.target); } });
  }, { threshold: 0.12 });
  document.querySelectorAll('.reveal').forEach(n => io.observe(n));

  document.querySelectorAll('.copy-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const text = qs('#' + btn.dataset.copy).textContent;
      navigator.clipboard.writeText(text).then(() => {
        const orig = btn.textContent; btn.textContent = 'Copied';
        setTimeout(() => { btn.textContent = orig; }, 1500);
      });
    });
  });
}

init();
