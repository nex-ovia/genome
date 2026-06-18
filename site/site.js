const qs = s => document.querySelector(s);
const el = (tag, cls, html) => { const e = document.createElement(tag); if (cls) e.className = cls; if (html != null) e.innerHTML = html; return e; };

async function init() {
    let data;
    try {
        const res = await fetch('content.json');
        data = await res.json();
    } catch (err) {
        console.error('content load failed', err);
        return;
    }
    renderSite(data);
    setupInteractions();
    loadReleases(data.repo, data.links.releases);
    if (window.lucide) window.lucide.createIcons();
}

function renderSite(data) {
    document.title = `${data.site.name} — ${data.site.tagline}`;

    // Nav
    const nav = qs('#nav-links');
    data.nav.forEach(item => {
        const a = el('a', 'hover:text-white transition-colors cursor-pointer');
        a.href = item.href; a.textContent = item.label;
        nav.appendChild(a);
    });
    const ghBtn = el('a', 'bg-white text-black px-6 py-2.5 rounded-full hover:bg-blue-600 hover:text-white transition-all font-bold text-[10px] uppercase ml-2');
    ghBtn.href = data.links.github; ghBtn.textContent = 'GitHub';
    nav.appendChild(ghBtn);

    // Hero
    qs('#hero-badge').innerHTML = `<i data-lucide="flask-conical" class="w-3.5 h-3.5"></i> ${data.hero.badge}`;
    qs('#hero-tagline').textContent = data.site.tagline;
    qs('#hero-desc').textContent = data.site.description;
    const actions = qs('#hero-actions');
    [[data.hero.cta_primary, 'btn-primary'], [data.hero.cta_secondary, 'btn-secondary']].forEach(([c, cls]) => {
        const a = el('a', cls, `<i data-lucide="${c.icon}" class="w-4 h-4"></i> ${c.label}`);
        a.href = c.href;
        actions.appendChild(a);
    });

    // Value grid
    const vg = qs('#value-grid');
    data.value.forEach(v => {
        vg.appendChild(el('div', 'glass-card rounded-[2.5rem] p-10 min-h-[380px] flex flex-col', `
            <div class="w-12 h-12 bg-blue-500/10 rounded-xl flex items-center justify-center text-blue-400 mb-8"><i data-lucide="${v.icon}"></i></div>
            <h4 class="text-2xl font-bold text-white mb-6 tracking-tight">${v.title}</h4>
            <div class="space-y-5">
                <div><span class="text-red-400/80 font-bold uppercase text-[10px] tracking-widest block mb-1">Problem</span><p class="text-gray-400 leading-relaxed">${v.problem}</p></div>
                <div class="pt-4 border-t border-white/5"><span class="text-emerald-400 font-bold uppercase text-[10px] tracking-widest block mb-1">genome</span><p class="text-gray-200 leading-relaxed">${v.solution}</p></div>
            </div>`));
    });

    // How grid
    const hg = qs('#how-grid');
    data.how.forEach((h, i) => {
        hg.appendChild(el('div', 'glass-card rounded-[2rem] p-8', `
            <div class="flex items-center gap-3 mb-5">
                <div class="w-10 h-10 bg-white/5 rounded-lg flex items-center justify-center text-blue-400"><i data-lucide="${h.icon}" class="w-5 h-5"></i></div>
                <span class="text-gray-600 font-bold text-sm">0${i + 1}</span>
            </div>
            <code class="text-blue-300 font-bold">genome ${h.step}</code>
            <p class="text-gray-400 leading-relaxed mt-3 text-sm">${h.desc}</p>`));
    });

    // Use cases
    const ug = qs('#use-grid');
    data.use.forEach(u => {
        ug.appendChild(el('div', 'glass-card rounded-[2rem] p-8 flex gap-5', `
            <div class="w-11 h-11 flex-shrink-0 bg-blue-500/10 rounded-xl flex items-center justify-center text-blue-400"><i data-lucide="${u.icon}"></i></div>
            <div><h4 class="text-xl font-bold text-white mb-2 tracking-tight">${u.title}</h4><p class="text-gray-400 leading-relaxed">${u.desc}</p></div>`));
    });

    // Install
    qs('#install-label').textContent = data.install.label;
    qs('#install-note').textContent = data.install.note;
    qs('#install-unix').textContent = data.install.unix;
    qs('#install-windows').textContent = data.install.windows;

    // Download header link
    qs('#all-releases').href = data.links.releases;

    // Footer
    qs('#footer-bio').textContent = data.site.description;
    const fl = qs('#footer-links');
    data.footer.sections.forEach(sec => {
        const links = sec.links.map(l => `<a href="${l.href}" class="text-gray-500 hover:text-white transition-colors font-medium">${l.label}</a>`).join('');
        fl.appendChild(el('div', '', `<div class="text-white font-bold uppercase tracking-widest text-xs mb-6">${sec.title}</div><div class="flex flex-col gap-4">${links}</div>`));
    });
    qs('#footer-copy').textContent = `${data.footer.tagline}  ·  Source-available under BSL 1.1`;
}

const PLATFORMS = [
    { match: 'x86_64-unknown-linux-musl', os: 'Linux', arch: 'x86_64', icon: 'monitor' },
    { match: 'aarch64-apple-darwin',      os: 'macOS', arch: 'Apple Silicon', icon: 'apple' },
    { match: 'x86_64-apple-darwin',       os: 'macOS', arch: 'Intel', icon: 'apple' },
    { match: 'x86_64-pc-windows-msvc',    os: 'Windows', arch: 'x86_64', icon: 'monitor' },
];

async function loadReleases(repo, releasesUrl) {
    const verEl = qs('#release-version');
    const listEl = qs('#release-list');
    try {
        const res = await fetch(`https://api.github.com/repos/${repo}/releases?per_page=1`);
        if (!res.ok) throw new Error('api ' + res.status);
        const rel = (await res.json())[0];
        if (!rel) throw new Error('no releases');
        verEl.innerHTML = `<i data-lucide="tag" class="w-4 h-4 text-blue-400"></i> Latest: <span class="text-white font-bold ml-1">${rel.tag_name}</span>${rel.prerelease ? ' <span class="text-amber-400 text-xs ml-2 uppercase tracking-widest">pre-release</span>' : ''}`;
        const binaries = rel.assets.filter(a => !a.name.endsWith('.sha256'));
        PLATFORMS.forEach(p => {
            const asset = binaries.find(a => a.name.includes(p.match));
            if (!asset) return;
            const mb = (asset.size / 1048576).toFixed(1);
            listEl.appendChild(el('div', 'release-row', `
                <div class="flex items-center gap-3"><i data-lucide="${p.icon}" class="w-4 h-4 text-gray-500"></i><span class="release-os">${p.os}</span> <span class="release-meta">${p.arch}</span></div>
                <div class="flex items-center gap-4"><span class="release-meta">${mb} MB</span><a href="${asset.browser_download_url}"><i data-lucide="download" class="w-3.5 h-3.5 inline"></i> Download</a></div>`));
        });
        if (window.lucide) window.lucide.createIcons();
    } catch (err) {
        verEl.textContent = 'Latest release';
        listEl.innerHTML = `<div class="release-row"><span class="text-gray-400">See all downloads on <a href="${releasesUrl}">GitHub Releases</a>.</span></div>`;
        if (window.lucide) window.lucide.createIcons();
    }
}

function setupInteractions() {
    const nav = qs('#navbar');
    window.addEventListener('scroll', () => {
        const s = window.scrollY > 50;
        nav.classList.toggle('nav-blur', s);
        nav.classList.toggle('py-4', s);
        nav.classList.toggle('py-8', !s);
    });
    document.querySelectorAll('.copy-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const text = qs('#' + btn.dataset.copy).textContent;
            navigator.clipboard.writeText(text).then(() => {
                const orig = btn.innerHTML;
                btn.innerHTML = '<i data-lucide="check" class="w-3.5 h-3.5"></i> Copied';
                if (window.lucide) window.lucide.createIcons();
                setTimeout(() => { btn.innerHTML = orig; if (window.lucide) window.lucide.createIcons(); }, 1600);
            });
        });
    });
}

init();
