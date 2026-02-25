import { Jimp } from 'jimp';

async function processImage() {
  const imagePath = process.argv[2];
  const outputPath = process.argv[3];

  const image = await Jimp.read(imagePath);
  
  // Basic background removal with simple anti-aliasing/feathering logic
  // We assume the background is pure white or very close to it.
  for (let idx = 0; idx < image.bitmap.data.length; idx += 4) {
    const r = image.bitmap.data[idx + 0];
    const g = image.bitmap.data[idx + 1];
    const b = image.bitmap.data[idx + 2];

    const brightness = (r + g + b) / 3;

    if (brightness > 245) {
      // Pure white = fully transparent
      image.bitmap.data[idx + 3] = 0; 
    } else if (brightness > 230) {
      // Very light grey/white edge = partially transparent
      const alpha = Math.max(0, Math.min(255, 255 - ((brightness - 230) * 17))); 
      image.bitmap.data[idx + 3] = Math.floor(alpha);
    }
  }

  await image.write(outputPath);
  console.log('Saved transparent logo to', outputPath);
}

processImage().catch(console.error);
