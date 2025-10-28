# Publishing Guide

## Prerequisites

1. **npm account**: You need an npm account. Create one at https://www.npmjs.com/signup
2. **npm login**: Run `npm login` to authenticate

## Steps to Publish

### 1. Update package.json

Before publishing, update these fields in `package.json`:

```json
{
  "name": "d-vecdb",  // or "@your-org/d-vecdb" for scoped package
  "version": "0.1.0",
  "repository": {
    "type": "git",
    "url": "https://github.com/YOUR_USERNAME/d-vecDB.git",
    "directory": "typescript-client"
  }
}
```

### 2. Build the project

```bash
npm run build
```

### 3. Test the package locally (optional)

```bash
# Create a tarball
npm pack

# This creates d-vecdb-0.1.0.tgz
# You can install this in another project to test:
# npm install /path/to/d-vecdb-0.1.0.tgz
```

### 4. Publish to npm

#### For a public package:

```bash
npm publish
```

#### For a scoped package (if name is @your-org/d-vecdb):

```bash
npm publish --access public
```

### 5. Verify the package

After publishing, verify at:
- https://www.npmjs.com/package/d-vecdb

## Updating the package

When you make changes:

1. Update the version in `package.json`:
   - Patch release (bug fixes): `0.1.0` → `0.1.1`
   - Minor release (new features): `0.1.0` → `0.2.0`
   - Major release (breaking changes): `0.1.0` → `1.0.0`

2. Or use npm version command:
   ```bash
   npm version patch  # 0.1.0 -> 0.1.1
   npm version minor  # 0.1.0 -> 0.2.0
   npm version major  # 0.1.0 -> 1.0.0
   ```

3. Rebuild and publish:
   ```bash
   npm run build
   npm publish
   ```

## Troubleshooting

### Package name already taken

If `d-vecdb` is already taken on npm:

1. Use a scoped package name in `package.json`:
   ```json
   {
     "name": "@your-username/d-vecdb"
   }
   ```

2. Publish with:
   ```bash
   npm publish --access public
   ```

### 401 Unauthorized

Run `npm login` to authenticate.

### Version already exists

Increment the version number in `package.json` before publishing.

## Alternative: Publish to GitHub Packages

You can also publish to GitHub Packages Registry:

1. Create `.npmrc` in project root:
   ```
   @YOUR_USERNAME:registry=https://npm.pkg.github.com
   ```

2. Update `package.json`:
   ```json
   {
     "name": "@YOUR_USERNAME/d-vecdb",
     "publishConfig": {
       "registry": "https://npm.pkg.github.com"
     }
   }
   ```

3. Authenticate with GitHub token:
   ```bash
   npm login --registry=https://npm.pkg.github.com
   ```

4. Publish:
   ```bash
   npm publish
   ```

## Post-Publishing

1. Update the README with the actual npm package link
2. Add installation instructions to main d-vecDB repository
3. Consider setting up automated publishing via GitHub Actions
4. Add npm badge to README:
   ```markdown
   [![npm version](https://badge.fury.io/js/d-vecdb.svg)](https://www.npmjs.com/package/d-vecdb)
   ```
