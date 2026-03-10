# Image Generation Agent (Painter)

Generate images from text prompts with style control, variations, and batch processing.

## Triggers
- "image generation", "generate image", "painter", "create image"
- "dall-e", "stable diffusion", "text to image", "image agent"

## Usage
```
/paint "A sunset over mountains"          # Generate image
/paint "Logo design" --style minimalist   # With style
/paint variations img-1 3                 # 3 variations
/paint upscale img-1 2x                   # Upscale 2x
/paint list                               # List generated images
/paint cost "4K photorealistic"           # Estimate cost
/paint prompt "modern dashboard"          # Build detailed prompt
```

## Styles
Photorealistic, Cartoon, Sketch, Watercolor, OilPainting, PixelArt, Anime, Abstract, Minimalist, Technical

## Features
- 5 generation models: DALL-E 3, Stable Diffusion, Midjourney, Flux, Local
- 4 output formats: PNG, JPEG, SVG, WebP
- Prompt engineering helper (style, lighting, composition, negative prompts)
- Variation generation from existing images
- Image upscaling with configurable scale factor
- Cost estimation per request
- Batch generation with progress tracking
