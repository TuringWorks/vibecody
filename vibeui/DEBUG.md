# VibeUI Debug Guide

## How to Debug the App

### Open DevTools

1. **In the running VibeUI window**, right-click anywhere
2. Select **"Inspect"** or **"Inspect Element"**
3. Or use keyboard shortcut:
   - Mac: `Cmd + Option + I`
   - Windows/Linux: `Ctrl + Shift + I`

### Check Console for Errors

1. Click the **"Console"** tab in DevTools
2. Look for any red error messages
3. Try clicking buttons and watch for console output

### Test Folder Picker

1. Click "📁 Open Folder" button
2. Watch console for these messages:
   - `"openFolder called"` - button was clicked
   - `"Calling dialog.open..."` - dialog function called
   - `"Dialog result: ..."` - dialog returned a value
   - `"Selected folder: ..."` - folder was selected

**If you see an error**, copy the full error message.

### Test AI Chat

1. Click "💬 AI Chat" button
2. Watch console for:
   - `"AI Chat button clicked, current state: false"` - button clicked
   - `"AI Chat toggled to: true"` - state changed
3. The chat panel should appear on the right side

**If nothing happens**, check if there are any errors in the console.

## Common Issues

### Folder Picker Not Working

**Symptoms:**
- Nothing happens when clicking "Open Folder"
- No dialog appears
- Error in console

**Possible Causes:**
1. Dialog plugin not loaded
2. Permission denied
3. JavaScript error

**Check:**
- Look for `"openFolder called"` in console
- If you see it, the button works
- If you don't see it, the click handler isn't firing

### AI Chat Not Appearing

**Symptoms:**
- Button clicks but panel doesn't appear
- No visual change

**Possible Causes:**
1. CSS issue (panel might be hidden)
2. State not updating
3. Component not rendering

**Check:**
- Look for `"AI Chat button clicked"` in console
- Check if `showAIChat` state changes
- Inspect the DOM to see if `<aside class="ai-chat-panel">` exists

## What to Report

If you find issues, please provide:

1. **Console errors** (copy full error text)
2. **Console logs** (what messages you see)
3. **What you clicked** (which button)
4. **What happened** (or didn't happen)
5. **Screenshot** of DevTools console (if possible)

## Quick Test

Run these in the DevTools Console:

```javascript
// Test if dialog plugin is available
console.log("Dialog plugin:", window.__TAURI__?.dialog);

// Test if invoke is available  
console.log("Invoke:", window.__TAURI__?.core?.invoke);
```

If either shows `undefined`, there's a plugin loading issue.
