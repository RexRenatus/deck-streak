# deepmerge 4.3.1, patched for GHSA-gcqx-w48m-w2h4

This directory is our own fixed copy of upstream `deepmerge@4.3.1`, the npm registry's release
(MIT, `license.txt`). pnpm installs it in place of the registry's release through one scoped
override in `pnpm-workspace.yaml`, `'@sveltejs/vite-plugin-svelte@7>deepmerge':
'file:./vendor/deepmerge'` (ADR-517, SPEC-499).

## Why

`deepmerge` 4.2.0 to 4.3.1 is affected by GHSA-gcqx-w48m-w2h4 (low). `mergeObject` assigns each
key with `destination[key] = ...`, so an own `__proto__` key in a merged source, the shape
`JSON.parse` makes, sets the merged object's prototype when the target is a primitive or has a
null prototype. No release fixes it: 4.3.1 is the registry's latest release, and the advisory
names no patched version. SvelteKit 3 requires `@sveltejs/vite-plugin-svelte` 7, and every 7.x
release declares `deepmerge ^4.3.1`, which merges the Mini App's build options.

## The fix it mirrors

Upstream pull request TehShrike/deepmerge#277
(<https://github.com/TehShrike/deepmerge/pull/277>), open when this copy was made. It adds a
`setProperty` helper that defines an own `__proto__` key as an own enumerable, writable and
configurable data property, assigns every other key as before, and uses it at the three
assignments in `mergeObject`. The copy applies that change to `index.js`, line for line #277's
hunks, and to the two built files in their own style: `dist/cjs.js`, the package's `main`, which
the Vite plugin loads, and `dist/umd.js`. Nothing else changes. `package.json`, `license.txt` and
`index.d.ts` are the release's bytes, so the package keeps its name and its version, 4.3.1; the
release's readme, changelog and build configuration are left out.

## The diff

Against the files of the registry's `deepmerge@4.3.1` tarball: 19 changed lines in each of the
three files, 57 in all.

```diff
--- deepmerge@4.3.1/index.js
+++ vendor/deepmerge/index.js
@@ -36,6 +36,19 @@
 	return Object.keys(target).concat(getEnumerableOwnPropertySymbols(target))
 }
 
+function setProperty(target, property, value) {
+	if (property === '__proto__') {
+		Object.defineProperty(target, property, {
+			value: value,
+			writable: true,
+			enumerable: true,
+			configurable: true
+		})
+	} else {
+		target[property] = value
+	}
+}
+
 function propertyIsOnObject(object, property) {
 	try {
 		return property in object
@@ -55,7 +68,7 @@
 	var destination = {}
 	if (options.isMergeableObject(target)) {
 		getKeys(target).forEach(function(key) {
-			destination[key] = cloneUnlessOtherwiseSpecified(target[key], options)
+			setProperty(destination, key, cloneUnlessOtherwiseSpecified(target[key], options))
 		})
 	}
 	getKeys(source).forEach(function(key) {
@@ -64,9 +77,9 @@
 		}
 
 		if (propertyIsOnObject(target, key) && options.isMergeableObject(source[key])) {
-			destination[key] = getMergeFunction(key, options)(target[key], source[key], options)
+			setProperty(destination, key, getMergeFunction(key, options)(target[key], source[key], options))
 		} else {
-			destination[key] = cloneUnlessOtherwiseSpecified(source[key], options)
+			setProperty(destination, key, cloneUnlessOtherwiseSpecified(source[key], options))
 		}
 	})
 	return destination
--- deepmerge@4.3.1/dist/cjs.js
+++ vendor/deepmerge/dist/cjs.js
@@ -61,6 +61,19 @@
 	return Object.keys(target).concat(getEnumerableOwnPropertySymbols(target))
 }
 
+function setProperty(target, property, value) {
+	if (property === '__proto__') {
+		Object.defineProperty(target, property, {
+			value: value,
+			writable: true,
+			enumerable: true,
+			configurable: true
+		});
+	} else {
+		target[property] = value;
+	}
+}
+
 function propertyIsOnObject(object, property) {
 	try {
 		return property in object
@@ -80,7 +93,7 @@
 	var destination = {};
 	if (options.isMergeableObject(target)) {
 		getKeys(target).forEach(function(key) {
-			destination[key] = cloneUnlessOtherwiseSpecified(target[key], options);
+			setProperty(destination, key, cloneUnlessOtherwiseSpecified(target[key], options));
 		});
 	}
 	getKeys(source).forEach(function(key) {
@@ -89,9 +102,9 @@
 		}
 
 		if (propertyIsOnObject(target, key) && options.isMergeableObject(source[key])) {
-			destination[key] = getMergeFunction(key, options)(target[key], source[key], options);
+			setProperty(destination, key, getMergeFunction(key, options)(target[key], source[key], options));
 		} else {
-			destination[key] = cloneUnlessOtherwiseSpecified(source[key], options);
+			setProperty(destination, key, cloneUnlessOtherwiseSpecified(source[key], options));
 		}
 	});
 	return destination
--- deepmerge@4.3.1/dist/umd.js
+++ vendor/deepmerge/dist/umd.js
@@ -65,6 +65,19 @@
 		return Object.keys(target).concat(getEnumerableOwnPropertySymbols(target))
 	}
 
+	function setProperty(target, property, value) {
+		if (property === '__proto__') {
+			Object.defineProperty(target, property, {
+				value: value,
+				writable: true,
+				enumerable: true,
+				configurable: true
+			});
+		} else {
+			target[property] = value;
+		}
+	}
+
 	function propertyIsOnObject(object, property) {
 		try {
 			return property in object
@@ -84,7 +97,7 @@
 		var destination = {};
 		if (options.isMergeableObject(target)) {
 			getKeys(target).forEach(function(key) {
-				destination[key] = cloneUnlessOtherwiseSpecified(target[key], options);
+				setProperty(destination, key, cloneUnlessOtherwiseSpecified(target[key], options));
 			});
 		}
 		getKeys(source).forEach(function(key) {
@@ -93,9 +106,9 @@
 			}
 
 			if (propertyIsOnObject(target, key) && options.isMergeableObject(source[key])) {
-				destination[key] = getMergeFunction(key, options)(target[key], source[key], options);
+				setProperty(destination, key, getMergeFunction(key, options)(target[key], source[key], options));
 			} else {
-				destination[key] = cloneUnlessOtherwiseSpecified(source[key], options);
+				setProperty(destination, key, cloneUnlessOtherwiseSpecified(source[key], options));
 			}
 		});
 		return destination
```

## The red-first test

`web/app/src/lib/deepmerge-prototype.test.ts` reaches deepmerge as `@sveltejs/vite-plugin-svelte`
resolves it, from that package's own installed directory. T1, T2 and T6 fail on the registry's
4.3.1, for the advisory's reason, and pass on this copy; T3 to T5 are controls that pass on both.
The red and green commits are recorded in `docs/red-first/SPEC-499.md`.

## The rule for leaving

This copy leaves when upstream publishes a `deepmerge` release outside the advisory's affected
range, `>= 4.2.0, <= 4.3.1`. The change that leaves removes the override's entry from
`pnpm-workspace.yaml`, deletes this directory, and regenerates the lock so that it resolves the
fixed release from the registry; the red-first test stays, and must still pass. Until then this
copy changes only to follow an upstream change to the same lines.
