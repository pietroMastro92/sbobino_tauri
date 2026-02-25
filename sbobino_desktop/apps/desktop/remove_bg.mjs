import { Jimp } from 'jimp';

async function processImage() {
  const imagePath = '/Users/pietromastro/.gemini/antigravity/brain/33f403cc-3ff4-403c-a183-e1034a7a92f9/sbobino_logo_1771931747778.png';
  const outputPath = '/tmp/sbobino_logo_transparent.png';

  const image = await Jimp.read(imagePath);
  
  for (let idx = 0; idx < image.bitmap.data.length; idx += 4) {
    const r = image.bitmap.data[idx + 0];
    const g = image.bitmap.data[idx + 1];
    const b = image.bitmap.data[idx + 2];

    // If pixel is very close to white, make it transparent
    if (r > 240 && g > 240 && b > 240) {
      image.bitmap.data[idx + 3] = 0; // Set alpha to 0 (transparent)
    }
  }

  await image.write(outputPath);
  console.log('Saved transparent logo to', outputPath);
}

processImage().catch(console.error);
