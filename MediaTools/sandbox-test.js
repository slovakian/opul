// Run by Scripts/test_media_tools.sh through Opul's qjs driver. Every line it prints must
// say the capability is missing, except the last, which proves console.log reaches stdout.
const globals = ['std', 'os', '__loadScript', 'require', 'Deno', 'process'];
for (const name of globals) {
  console.log(`${name}: ${typeof globalThis[name] === 'undefined' ? 'missing' : 'PRESENT'}`);
}

const modules = ['os', 'std', 'qjs:os', 'qjs:std', '/etc/hosts'];
Promise.allSettled(modules.map((name) => import(name))).then((results) => {
  results.forEach((result, index) => {
    console.log(`import ${modules[index]}: ${result.status === 'rejected' ? 'missing' : 'PRESENT'}`);
  });
  print('stdout: ok');
});
