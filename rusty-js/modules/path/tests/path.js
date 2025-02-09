// Simple assertion function
function assert(condition, message) {
    if (!condition) {
        print('Assertion failed: ' + message);
        throw message || 'Assertion failed';
    }
}

function assertEqual(actual, expected) {
    if (actual !== expected) {
        let message = 'Expected "' + expected + '" but got "' + actual + '"';
        print('Assertion failed: ' + message);
        throw message;
    }
    return true;
}

// Test results will be stored here
const results = {
    total: 0,
    passed: 0,
    failed: []
};

try {
    // Test basename
    results.total += 5;
    print('Testing basename...');
    results.passed += assertEqual(path.basename('/foo/bar/baz.html'), 'baz.html');
    results.passed += assertEqual(path.basename('/foo/bar/baz.html', '.html'), 'baz');
    results.passed += assertEqual(path.basename('/foo/bar/baz'), 'baz');
    results.passed += assertEqual(path.basename('/'), '');
    results.passed += assertEqual(path.basename(''), '');

    // Test dirname
    results.total += 5;
    print('Testing dirname...');
    results.passed += assertEqual(path.dirname('/foo/bar/baz'), '/foo/bar');
    results.passed += assertEqual(path.dirname('/foo/bar/baz/'), '/foo/bar');
    results.passed += assertEqual(path.dirname('/foo'), '/');
    results.passed += assertEqual(path.dirname('foo'), '.');
    results.passed += assertEqual(path.dirname(''), '.');

    // Test extname
    results.total += 5;
    print('Testing extname...');
    results.passed += assertEqual(path.extname('index.html'), '.html');
    results.passed += assertEqual(path.extname('index.coffee.md'), '.md');
    results.passed += assertEqual(path.extname('index.'), '.');
    results.passed += assertEqual(path.extname('index'), '');
    results.passed += assertEqual(path.extname('.index'), '');

    // Test isAbsolute
    results.total += 3;
    print('Testing isAbsolute...');
    results.passed += assertEqual(path.isAbsolute('/foo/bar'), true);
    results.passed += assertEqual(path.isAbsolute('foo/bar'), false);
    results.passed += assertEqual(path.isAbsolute('./foo/bar'), false);

    // Test join
    results.total += 4;
    print('Testing join...');
    results.passed += assertEqual(path.join('/foo', 'bar', 'baz'), '/foo/bar/baz');
    results.passed += assertEqual(path.join('/foo', 'bar', '../baz'), '/foo/baz');
    results.passed += assertEqual(path.join('foo', 'bar', 'baz'), 'foo/bar/baz');
    results.passed += assertEqual(path.join(''), '.');

    // Test normalize
    results.total += 4;
    print('Testing normalize...');
    results.passed += assertEqual(path.normalize('/foo/bar//baz/asdf/quux/..'), '/foo/bar/baz/asdf');
    results.passed += assertEqual(path.normalize('foo/bar//baz/asdf/quux/..'), 'foo/bar/baz/asdf');
    results.passed += assertEqual(path.normalize('/foo/../bar'), '/bar');
    results.passed += assertEqual(path.normalize('foo/..'), '.');

    // Test parse
    results.total += 5;
    print('Testing parse...');
    const parsed = path.parse('/home/user/dir/file.txt');
    results.passed += assertEqual(parsed.root, '/');
    results.passed += assertEqual(parsed.dir, '/home/user/dir');
    results.passed += assertEqual(parsed.base, 'file.txt');
    results.passed += assertEqual(parsed.ext, '.txt');
    results.passed += assertEqual(parsed.name, 'file');

    // Test format
    results.total += 2;
    print('Testing format...');
    results.passed += assertEqual(
        path.format({
            root: '/',
            dir: '/home/user/dir',
            base: 'file.txt'
        }),
        '/home/user/dir/file.txt'
    );
    results.passed += assertEqual(
        path.format({
            dir: '/home/user/dir',
            name: 'file',
            ext: '.txt'
        }),
        '/home/user/dir/file.txt'
    );

    // Test platform specific
    results.total += 2;
    print('Testing platform specific...');
    results.passed += assertEqual(path.sep, '/');
    results.passed += assertEqual(path.delimiter, ':');

    // Test edge cases
    results.total += 2;
    print('Testing edge cases...');
    results.passed += assertEqual(path.normalize(''), '.');
    results.passed += assertEqual(path.join('', ''), '.');

    // Test Unicode support
    results.total += 3;
    print('Testing Unicode support...');
    results.passed += assertEqual(path.basename('/foo/bar/文件.txt'), '文件.txt');
    results.passed += assertEqual(path.basename('/foo/bar/文件.txt', '.txt'), '文件');
    results.passed += assertEqual(path.extname('文件.txt'), '.txt');

    print(`Tests completed: ${results.passed}/${results.total} passed`);

} catch (err) {
    print('Test failed with error: ' + JSON.stringify(err));
    results.failed.push(err.toString());
}

// Return test results
({
    total: results.total,
    passed: results.passed,
    failed: results.failed,
    success: results.failed.length === 0 && results.total === results.passed
})
