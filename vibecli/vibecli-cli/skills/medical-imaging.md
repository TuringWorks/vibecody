---
triggers: ["medical imaging", "DICOM", "NIfTI", "CT scan", "MRI", "X-ray", "radiology", "segmentation", "ITK", "SimpleITK", "nibabel", "pydicom", "3D Slicer", "MONAI"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["python3"]
category: scientific
---

# Medical Imaging

When working with medical imaging pipelines (DICOM, MRI, CT, X-ray):

1. Load DICOM files with `pydicom`: `ds = pydicom.dcmread('image.dcm')` — access pixel data with `ds.pixel_array` and metadata with `ds.PatientName`, `ds.Modality`, `ds.SliceThickness`.
2. Use `SimpleITK` for 3D image processing — `sitk.ReadImage('volume.nii.gz')` handles NIfTI, DICOM series, MetaImage, and NRRD; preserves spatial metadata (origin, spacing, direction).
3. For DICOM series: `reader = sitk.ImageSeriesReader(); reader.SetFileNames(sitk.ImageSeriesReader.GetGDCMSeriesFileNames(directory))` — loads a full CT/MRI volume from a folder of slices.
4. Use `nibabel` for neuroimaging NIfTI files: `img = nib.load('brain.nii.gz'); data = img.get_fdata()` — preserves affine transform between voxel and world coordinates.
5. Apply windowing for CT visualization: `window_center, window_width = 40, 400` (soft tissue); convert Hounsfield Units to display range with `np.clip((hu - center + width/2) / width * 255, 0, 255)`.
6. Use `MONAI` for deep learning medical image analysis — `monai.transforms.Compose([LoadImage, EnsureChannelFirst, Spacing, ScaleIntensity, RandRotate90])` builds reproducible preprocessing pipelines.
7. For organ/lesion segmentation: use U-Net architectures — `monai.networks.nets.UNet(spatial_dims=3, in_channels=1, out_channels=2, channels=(16,32,64,128), strides=(2,2,2))`.
8. Evaluate segmentation with Dice Score: `monai.metrics.DiceMetric(include_background=False)` — report per-class Dice and Hausdorff Distance for clinical relevance.
9. Handle anonymization: `pydicom` can remove PHI fields — iterate `ds.walk(callback)` to strip `PatientName`, `PatientID`, `InstitutionName`, `StudyDate` per HIPAA Safe Harbor rules.
10. Use `scikit-image` for classical image processing: `skimage.filters.threshold_otsu()` for auto-thresholding, `measure.regionprops()` for connected component analysis, `morphology.remove_small_objects()` for cleanup.
11. Register images with `SimpleITK`: `sitk.ImageRegistrationMethod()` supports rigid, affine, and deformable (B-spline) registration — use mutual information metric for multi-modal (CT-to-MRI) alignment.
12. Use `VTK` or `3D Slicer` (via `SlicerPython`) for 3D volume rendering and surgical planning visualization — export meshes with `vtk.vtkMarchingCubes` for surface extraction from segmentation masks.
13. Always preserve DICOM coordinate systems — use `ImagePositionPatient` and `ImageOrientationPatient` for correct physical-space alignment; never rely on array indices alone.
14. For data augmentation: use `monai.transforms.RandAffine`, `RandGaussianNoise`, `RandFlip` — medical imaging requires domain-aware augmentation (no vertical flips for chest X-rays).
15. Store processed datasets in NIfTI format with `nib.save(nib.Nifti1Image(data, affine), 'output.nii.gz')` — compressed NIfTI is the standard interchange format for research pipelines.
