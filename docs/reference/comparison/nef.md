---
title: NEF Compatibility
---

# NEF Compatibility Report

**Generated:** 2025-12-08 | **ExifTool:** v13.43 | **OxiDex:** v1.2.1

## Summary

- **Files Tested:** 1
- **Coverage:** 12.7%
- **Matched Tags:** 26
- **Missing Tags:** 165
- **Extra Tags:** 33
- **Value Differences:** 13

## Value Differences

Tags where ExifTool and OxiDex extract different values:

| Tag | ExifTool | OxiDex |
|-----|----------|--------|
| `EXIF:BitsPerSample` | 12 | 8 |
| `EXIF:CFAPattern` | [Blue,Green][Green,Red] | [Binary data] |
| `EXIF:Compression` | Nikon NEF Compressed | Uncompressed |
| `EXIF:ExposureCompensation` | 0 | [Binary data] |
| `EXIF:ExposureTime` | 1/20 | 0.05 |
| `EXIF:ImageHeight` | 2014 | 106 |
| `EXIF:ImageWidth` | 3040 | 160 |
| `EXIF:PhotometricInterpretation` | Color Filter Array | RGB |
| `EXIF:RowsPerStrip` | 2014 | 106 |
| `EXIF:SamplesPerPixel` | 1 | 3 |
| `EXIF:SceneType` | Directly photographed | [Binary data] |
| `EXIF:StripOffsets` | 1376 | 6170 |
| `EXIF:SubfileType` | Full-resolution image | Reduced-resolution Image |

## Missing Tags

Tags ExifTool extracts that OxiDex doesn't:

| Tag | Sample Value |
|-----|-------------|
| `EXIF:CFAPattern2` | 2 1 1 0 |
| `EXIF:CFARepeatPatternDim` | 2 2 |
| `EXIF:ExposureProgram` | Aperture-priority AE |
| `EXIF:JpgFromRaw` | (Binary data 29 bytes, use -b option to extract) |
| `EXIF:JpgFromRawLength` | 29 |
| `EXIF:JpgFromRawStart` | 1120 |
| `EXIF:TIFF-EPStandardID` | 1.0.0.0 |
| `ICC_Profile:BlueMatrixColumn` | 0.1492 0.06322 0.74463 |
| `ICC_Profile:BlueTRC` | (Binary data 14 bytes, use -b option to extract) |
| `ICC_Profile:CMMFlags` | Not Embedded, Independent |
| `ICC_Profile:ColorSpaceData` | RGB  |
| `ICC_Profile:ConnectionSpaceIlluminant` | 0.9642 1 0.82491 |
| `ICC_Profile:DeviceAttributes` | Reflective, Glossy, Positive, Color |
| `ICC_Profile:DeviceManufacturer` | none |
| `ICC_Profile:DeviceModel` |  |
| `ICC_Profile:GreenMatrixColumn` | 0.20525 0.62566 0.06087 |
| `ICC_Profile:GreenTRC` | (Binary data 14 bytes, use -b option to extract) |
| `ICC_Profile:MediaWhitePoint` | 0.9505 1 1.0891 |
| `ICC_Profile:PrimaryPlatform` | Apple Computer Inc. |
| `ICC_Profile:ProfileCMMType` | Nikon Corporation |
| `ICC_Profile:ProfileClass` | Display Device Profile |
| `ICC_Profile:ProfileConnectionSpace` | XYZ  |
| `ICC_Profile:ProfileCopyright` | Nikon Inc. & Nikon Corporation 2001 |
| `ICC_Profile:ProfileCreator` |  |
| `ICC_Profile:ProfileDateTime` | 1999:12:07 18:59:22 |
| `ICC_Profile:ProfileDescription` | Nikon Adobe RGB 4.0.0.3000 |
| `ICC_Profile:ProfileFileSignature` | acsp |
| `ICC_Profile:ProfileID` | 0 |
| `ICC_Profile:ProfileVersion` | 2.2.0 |
| `ICC_Profile:RedMatrixColumn` | 0.60976 0.31113 0.01947 |
| `ICC_Profile:RedTRC` | (Binary data 14 bytes, use -b option to extract) |
| `ICC_Profile:RenderingIntent` | Perceptual |
| `IPTC:ApplicationRecordVersion` | 4 |
| `IPTC:Caption-Abstract` | A caption |
| `IPTC:City` | Kingston |
| `IPTC:Country-PrimaryLocationName` | Canada |
| `IPTC:Province-State` | Ontario |
| `IPTC:SpecialInstructions` | none |
| `MakerNotes:AFAperture` | 3.6 |
| `MakerNotes:AFAreaMode` | Single Area |
| `MakerNotes:AFPoint` | Center |
| `MakerNotes:AFPointsInFocus` | Center |
| `MakerNotes:AdvancedRaw` | On |
| `MakerNotes:AutoRedEye` | On |
| `MakerNotes:BitDepth` | 8 |
| `MakerNotes:BrightnessAdj` | 0 |
| `MakerNotes:ColorAberrationControl` | Off |
| `MakerNotes:ColorBalanceAdj` | On |
| `MakerNotes:ColorBoostLevel` | 10 |
| `MakerNotes:ColorBoostType` | People |
| `MakerNotes:ColorBooster` | Off |
| `MakerNotes:ColorGain` | 0.00 0.00 0.00 |
| `MakerNotes:ColorHue` | Mode2 |
| `MakerNotes:ColorMoireReductionMode` | Off |
| `MakerNotes:Compression` | JPEG (old-style) |
| `MakerNotes:ContrastCurve` | (Binary data 17 bytes, use -b option to extract) |
| `MakerNotes:CropBottom` | 2000 |
| `MakerNotes:CropLeft` | 0 |
| `MakerNotes:CropOutputHeight` | 2000 |
| `MakerNotes:CropOutputHeightInches` | 6.66666666666667 |
| `MakerNotes:CropOutputPixels` | 6016000 |
| `MakerNotes:CropOutputResolution` | 300 |
| `MakerNotes:CropOutputScale` | 1 |
| `MakerNotes:CropOutputWidth` | 3008 |
| `MakerNotes:CropOutputWidthInches` | 10.0266666666667 |
| `MakerNotes:CropRight` | 3008 |
| `MakerNotes:CropScaledResolution` | 300 |
| `MakerNotes:CropSourceResolution` | 300 |
| `MakerNotes:CropTop` | 0 |
| `MakerNotes:Curves` | On |
| `MakerNotes:D-LightingHQ` | Off |
| `MakerNotes:D-LightingHQColorBoost` | 60 |
| `MakerNotes:D-LightingHQHighlight` | 1 |
| `MakerNotes:D-LightingHQSelected` | No |
| `MakerNotes:D-LightingHQShadow` | 50 |
| `MakerNotes:D-LightingHS` | Off |
| `MakerNotes:D-LightingHSAdjustment` | 25 |
| `MakerNotes:D-LightingHSColorBoost` | 60 |
| `MakerNotes:DigitalICE` | Normal |
| `MakerNotes:EdgeNoiseReduction` | Off |
| `MakerNotes:EffectiveMaxAperture` | 3.6 |
| `MakerNotes:EnhanceDarkTones` | Off |
| `MakerNotes:ExitPupilPosition` | 102.4 mm |
| `MakerNotes:ExposureAdj` | 0.3 |
| `MakerNotes:ExposureAdj2` | 0.3 |
| `MakerNotes:ExposureBracketValue` | 0 |
| `MakerNotes:ExposureDifference` | 0 |
| `MakerNotes:FilmType` | POSITIVE        |
| `MakerNotes:FlashMode` | Did Not Fire |
| `MakerNotes:FlipHorizontal` | No |
| `MakerNotes:FocalLength` | 18.3 mm |
| `MakerNotes:FocusDistance` | 2.37 m |
| `MakerNotes:FocusMode` | AF-S |
| `MakerNotes:FocusPosition` | 0xf1 |
| `MakerNotes:HueAdjustment` | 0 |
| `MakerNotes:IFD0_Offset` | 8 |
| `MakerNotes:ISO` | 200 |
| `MakerNotes:ISOSetting` | 200 |
| `MakerNotes:ImageDustOff` | On |
| `MakerNotes:ImageOptimization` | Custom |

*...and 65 more missing tags*

## Extra Tags

Tags OxiDex extracts that ExifTool doesn't:

| Tag | Value |
|-----|-------|
| `Composite:Aperture` | 3.5 |
| `Composite:ISO` | 200 |
| `Composite:ImageSize` | 160x106 |
| `Composite:Megapixels` | 0.017 |
| `Composite:ShutterSpeed` | 0.05 |
| `EXIF:0x83BB` | 540 |
| `EXIF:0x8822` | 3 |
| `EXIF:0x9216` | [Binary data] |
| `EXIF:ICC_Profile` | [Binary data] |
| `NEF:HasRAWData` | true |
| `NEF:ImageLayerCount` | 2 |
| `NEF:RAWBitDepth` | 12 |
| `NEF:RAWCompression` | Nikon Lossless Compressed |
| `NEF:RAWImageSize` | 3040x2014 |
| `SubIFD0:0x0201` | 1120 |
| `SubIFD0:0x0202` | 29 |
| `SubIFD0:0x828D` | 2 |
| `SubIFD0:BitsPerSample` | 12 |
| `SubIFD0:CFAPattern2` | [Binary data] |
| `SubIFD0:Compression` | Nikon NEF Compressed |
| `SubIFD0:ImageHeight` | 2014 |
| `SubIFD0:ImageWidth` | 3040 |
| `SubIFD0:Orientation` | Horizontal (normal) |
| `SubIFD0:PhotometricInterpretation` | Color Filter Array |
| `SubIFD0:PlanarConfiguration` | Chunky |
| `SubIFD0:ResolutionUnit` | inches |
| `SubIFD0:RowsPerStrip` | 2014 |
| `SubIFD0:SamplesPerPixel` | 1 |
| `SubIFD0:StripByteCounts` | 18 |
| `SubIFD0:StripOffsets` | 1376 |
| `SubIFD0:SubfileType` | Full-resolution Image |
| `SubIFD0:XResolution` | 300 |
| `SubIFD0:YResolution` | 300 |

---

[← Back to Overview](./)
