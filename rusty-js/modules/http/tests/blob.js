// Simple assertion functions
function assert(condition, message) {
    if (!condition) {
        print('Assertion failed: ' + message);
        throw message || 'Assertion failed';
    }
    return true;
}

function assertEqual(actual, expected, message) {
    if (actual != expected) {
        let error = `Expected value "${expected}" but got "${actual}"${message ? ': ' + message : ''}`;
        print('Assertion failed: ' + error);
        throw error;
    }
    return true;
}

async function assertArrayBufferEquals(arrayBuffer1, arrayBuffer2, message) {
    const view1 = new Uint8Array(arrayBuffer1);
    const view2 = new Uint8Array(arrayBuffer2);

    if (view1.length !== view2.length) {
        let error = `ArrayBuffer length mismatch: ${view1.length} !== ${view2.length}${message ? ': ' + message : ''}`;
        print('Assertion failed: ' + error);
        throw error;
    }

    for (let i = 0; i < view1.length; i++) {
        if (view1[i] !== view2[i]) {
            let error = `ArrayBuffer content mismatch at position ${i}: ${view1[i]} !== ${view2[i]}${message ? ': ' + message : ''}`;
            print('Assertion failed: ' + error);
            throw error;
        }
    }
    return 1;
}

// Test results storage
const results = {
    total: 0,
    passed: 0,
    failed: []
};

// Run tests and return results
(async function() {
    try {
        // Test constructor
        results.total += 3;
        print('Testing constructor...');
        const emptyBlob = new Blob();
        results.passed += assert(emptyBlob.size == 0, 'Empty Blob should have size 0');
        results.passed += assert(emptyBlob.type === '', 'Empty Blob should have empty type');

        const textBlob = new Blob(['Hello World'], { type: 'text/plain' });
        results.passed += assert(textBlob.type === 'text/plain', 'Blob type should be set correctly');

        // Test type property
        results.total += 2;
        print('Testing type property...');
        const invalidType = 'invalid/' + String.fromCharCode(1) + 'type';
        const invalidTypeBlob = new Blob([], { type: invalidType });
        results.passed += assert(invalidTypeBlob.type === '', 'Invalid MIME type should be converted to empty string');

        const upperCaseTypeBlob = new Blob([], { type: 'TEXT/PLAIN' });
        results.passed += assert(upperCaseTypeBlob.type === 'text/plain', 'MIME type should be converted to lowercase');

        // Test size property
        results.total += 2;
        print('Testing size property...');
        const binaryBlob = new Blob([new Uint8Array([1, 2, 3, 4])]);
        results.passed += assert(binaryBlob.size == 4, 'Blob size should reflect binary data size');

        const multiPartBlob = new Blob(['Hello', new Uint8Array([1, 2, 3])]);
        results.passed += assert(multiPartBlob.size == 8, 'Blob size should calculate multiple parts correctly');

        // Test slice method
        results.total += 4;
        print('Testing slice method...');
        const originalBlob = new Blob(['Hello World']);
        const slicedBlob = originalBlob.slice(0, 5, 'text/plain');
        results.passed += assert(slicedBlob.size == 5, 'Slice should create Blob with correct size');
        const slicedText = await slicedBlob.text();
        results.passed += assert(slicedText === 'Hello', 'Slice should contain correct content');

        const negativeSlice = originalBlob.slice(-5);
        const negativeText = await negativeSlice.text();
        results.passed += assert(negativeText === 'World', 'Negative index slice should work correctly');

        const emptySlice = originalBlob.slice(5, 1);
        results.passed += assert(emptySlice.size == 0, 'Invalid range slice should create empty Blob');

        // Test arrayBuffer method
        results.total += 2;
        print('Testing arrayBuffer method...');
        const arrayBuffer1 = await textBlob.arrayBuffer();
        const expectedArray = new TextEncoder().encode('Hello World');
        results.passed += await assertArrayBufferEquals(arrayBuffer1, expectedArray.buffer, 'arrayBuffer should return correct data');

        const emptyBuffer = await emptyBlob.arrayBuffer();
        results.passed += assert(new Uint8Array(emptyBuffer).length == 0, 'Empty Blob should have empty arrayBuffer');

        // Test text method
        results.total += 2;
        print('Testing text method...');
        const textContent = await textBlob.text();
        results.passed += assert(textContent === 'Hello World', 'text should return correct string');

        const emptyText = await emptyBlob.text();
        results.passed += assert(emptyText === '', 'Empty Blob should return empty string');

        // Test line ending handling
        results.total += 2;
        print('Testing line ending handling...');
        const crlfText = 'line1\r\nline2\nline3\rline4';
        const transparentBlob = new Blob([crlfText], { endings: 'transparent' });
        const transparentText = await transparentBlob.text();
        results.passed += assert(transparentText === crlfText, 'transparent mode should preserve original line endings');

        const nativeBlob = new Blob([crlfText], { endings: 'native' });
        const nativeText = await nativeBlob.text();
        const expectedLines = crlfText.split(/\r\n|\r|\n/).join(process.platform === 'win32' ? '\r\n' : '\n');
        results.passed += assert(nativeText === expectedLines, 'native mode should convert to platform-specific line endings');

        // Test Blob chaining and complex compositions
        results.total += 4;
        print('Testing Blob chaining and compositions...');
        
        // Test Blob in Blob
        const sourceBlob = new Blob(['Hello']);
        print('Created source blob');
        const chainedBlob = new Blob([sourceBlob, ' World']);
        print('Created chained blob');
        const chainedText = await chainedBlob.text();
        print('Chained blob text:', chainedText);
        results.passed += assert(chainedText === 'Hello World', 'Blob chaining should work correctly');

        // Test mixed content with Blob, TypedArray, and ArrayBuffer
        const typedArray = new Uint8Array([32]); // space in ASCII
        const mixedPartsBlob = new Blob([
            sourceBlob,
            typedArray,
            'World'
        ]);
        print('Created mixed parts blob');
        const mixedText = await mixedPartsBlob.text();
        print('Mixed parts blob text:', mixedText);
        results.passed += assert(mixedText === 'Hello World', 'Mixed content blob should work correctly');

        // Test nested Blobs
        const nestedBlob = new Blob([
            new Blob(['Nested']),
            new Uint8Array([32]),
            new Blob(['Content'])
        ]);
        print('Created nested blob');
        const nestedText = await nestedBlob.text();
        print('Nested blob text:', nestedText);
        results.passed += assert(nestedText === 'Nested Content', 'Nested blobs should work correctly');

        // Test ArrayBuffer in Blob
        const testBuffer = new ArrayBuffer(5);
        const view = new Uint8Array(testBuffer);
        view.set([72, 101, 108, 108, 111]); // "Hello" in ASCII
        const bufferBlob = new Blob([testBuffer, new Uint8Array([32]), 'World']);
        print('Created buffer blob');
        const bufferText = await bufferBlob.text();
        print('Buffer blob text:', bufferText);
        results.passed += assert(bufferText === 'Hello World', 'ArrayBuffer in Blob should work correctly');

        print(`Tests completed: ${results.passed}/${results.total} passed`);

    } catch (err) {
        print('Test failed with error: ' + err);
        print('Error details:', err instanceof Error ? err.stack : 'No stack trace available');
        results.failed.push(err.toString());
    }

    return {
        total: results.total,
        passed: results.passed,
        failed: results.failed,
        success: results.failed.length === 0 && results.total === results.passed
    };
})();
