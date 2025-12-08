---
title: NEF Compatibility
---

# NEF Compatibility Report

**Generated:** 2025-12-08 | **ExifTool:** v13.43 | **OxiDex:** v1.2.1

## Summary

- **Files Tested:** 1
- **Coverage:** 19.6%
- **Matched Tags:** 40
- **Missing Tags:** 137
- **Extra Tags:** 36
- **Value Differences:** 27

## Value Differences

Tags where ExifTool and OxiDex extract different values:

| Tag | ExifTool | OxiDex |
|-----|----------|--------|
| `EXIF:BitsPerSample` | 12 | 8 |
| `EXIF:Compression` | Nikon NEF Compressed | Uncompressed |
| `EXIF:ExposureCompensation` | 0 | (Binary data 8 bytes, use -b option to e... |
| `EXIF:ExposureTime` | 1/20 | 0.05 |
| `EXIF:ImageHeight` | 2014 | 106 |
| `EXIF:ImageWidth` | 3040 | 160 |
| `EXIF:PhotometricInterpretation` | Color Filter Array | RGB |
| `EXIF:RowsPerStrip` | 2014 | 106 |
| `EXIF:SamplesPerPixel` | 1 | 3 |
| `EXIF:StripOffsets` | 1376 | 6170 |
| `EXIF:SubfileType` | Full-resolution image | Reduced-resolution Image |
| `MakerNotes:ExposureBracketValue` | 0 | +88.7 EV |
| `MakerNotes:FocalLength` | 18.3 mm | 12592 mm |
| `MakerNotes:FocusDistance` | 2.37 m | 12592 mm |
| `MakerNotes:FocusMode` | AF-S | Unknown |
| `MakerNotes:ISOSetting` | 200 | ISO 13107200 |
| `MakerNotes:LensFStops` | 5.33 | 65562.7 |
| `MakerNotes:LensType` | G | 0x06 |
| `MakerNotes:MakerNoteVersion` | 2.1 | 0210 |
| `MakerNotes:MaxApertureAtMaxFocal` | 4.5 | f/1135.6 |
| `MakerNotes:MaxApertureAtMinFocal` | 3.6 | f/1158.4 |
| `MakerNotes:NoiseReduction` | On | OFF |
| `MakerNotes:Quality` | RAW | Unknown |
| `MakerNotes:SensorPixelSize` | 7.8 x 7.8 um | 0x00000332 |
| `MakerNotes:Sharpness` | None | 516 |
| `MakerNotes:ShootingMode` | Single-Frame | Single Frame |
| `MakerNotes:WhiteBalance` | Auto | Unknown |

## Missing Tags

Tags ExifTool extracts that OxiDex doesn't:

| Tag | Sample Value |
|-----|-------------|
| `EXIF:CFAPattern2` | 2 1 1 0 |
| `EXIF:CFARepeatPatternDim` | 2 2 |
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
| `MakerNotes:ExposureDifference` | 0 |
| `MakerNotes:FilmType` | POSITIVE        |
| `MakerNotes:FlipHorizontal` | No |
| `MakerNotes:FocusPosition` | 0xf1 |
| `MakerNotes:IFD0_Offset` | 8 |
| `MakerNotes:ISO` | 200 |
| `MakerNotes:ImageDustOff` | On |
| `MakerNotes:LCHEditor` | On |
| `MakerNotes:Lens` | 18-70mm f/3.5-4.5 |
| `MakerNotes:LensDataVersion` | 0101 |
| `MakerNotes:LensIDNumber` | 127 |
| `MakerNotes:MCUVersion` | 132 |
| `MakerNotes:MasterGain` | 0.0 |
| `MakerNotes:MaxFocalLength` | 71.3 mm |
| `MakerNotes:MinFocalLength` | 18.3 mm |
| `MakerNotes:NEFLinearizationTable` | (Binary data 17 bytes, use -b option to extract) |
| `MakerNotes:NoiseReductionIntensity` | 0 |

*...and 37 more missing tags*

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
| `EXIF:0x9216` | (Binary data 4 bytes, use -b option to extract) |
| `EXIF:ICC_Profile` | (Binary data 492 bytes, use -b option to extract) |
| `MakerNotes:ISOSelection` | Manual |
| `MakerNotes:ISOSpeed` | ISO 13107200 |
| `MakerNotes:LensID` | Unknown (11284) |
| `MakerNotes:WB_RBLevels` | 17989 25376 |
| `NEF:HasRAWData` | true |
| `NEF:ImageLayerCount` | 2 |
| `NEF:RAWBitDepth` | 12 |
| `NEF:RAWCompression` | Nikon Lossless Compressed |
| `NEF:RAWImageSize` | 3040x2014 |
| `SubIFD0:0x0201` | 1120 |
| `SubIFD0:0x0202` | 29 |
| `SubIFD0:0x828D` | 2 |
| `SubIFD0:BitsPerSample` | 12 |
| `SubIFD0:CFAPattern2` | [Blue,Green][Green,Red] |
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
