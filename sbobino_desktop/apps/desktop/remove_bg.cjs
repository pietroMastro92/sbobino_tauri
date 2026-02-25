const Jimp = require('jimp');

async function processImage() {
  const imagePath = '/Users/pietromastro/.gemini/antigravity/brain/33f403cc-3ff4-403c-a183-e1034a7a92f9/sbobino_logo_1771931747778.png';
  const outputPath = '/tmp/sbobino_logo_transparent.png';

  const image = await Jimp.read(imagePath);
  
  image.scan(0, 0, image.bitmap.width, image.bitmap.height, function (x, y, idx) {
    const r = this.bitmap.data[idx + 0];
    const g = this.bitmap.data[idx + 1];
    const b = this.bitmap.data[idx + 2];

    // If pixel is very close to white, make it transparent
    if (r > 245 && g > 245 && b > 245) {
      this.bitmap.data[idx + 3] = 0; // Set alpha to 0 (transparent)
    }
  });

  await image.writeAsync(outputPath);
  console.log('Saved transparent logo to', outputPath);
}

processImage().catch(console.error);
