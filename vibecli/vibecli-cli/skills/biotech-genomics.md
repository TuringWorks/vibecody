---
triggers: ["genomics", "bioinformatics", "DNA", "sequencing", "FASTA", "VCF", "gene expression", "variant calling", "genome assembly", "phylogenetics"]
tools_allowed: ["read_file", "write_file", "bash"]
category: biotech
---

# Biotech Genomics and Bioinformatics

When working with genomics data, bioinformatics pipelines, and biological sequence analysis:

1. Process raw sequencing reads by first running quality control with FastQC, then trim adapters and low-quality bases with Trimmomatic or fastp; always verify per-base quality scores, adapter contamination rates, and GC content distribution before proceeding to alignment.

2. Align reads to a reference genome using BWA-MEM2 for DNA or STAR for RNA-seq; index the reference genome once and reuse, output sorted BAM files, and mark duplicates with Picard or samblaster to prevent PCR artifacts from inflating variant calls.

3. Implement variant calling workflows following GATK Best Practices: base quality score recalibration (BQSR), call variants with HaplotypeCaller in GVCF mode for cohort analysis, consolidate with GenomicsDBImport, and joint-genotype with GenotypeGVCFs for population-scale studies.

4. Parse and manipulate VCF files using bcftools or cyvcf2 (Python); filter variants by quality (QUAL >= 30), read depth (DP >= 10), allele frequency, and functional annotation; always preserve the VCF header and validate against the VCF specification after transformations.

5. Annotate variants with functional impact using tools like VEP (Ensembl Variant Effect Predictor) or SnpEff; cross-reference against ClinVar, gnomAD population frequencies, and COSMIC for somatic mutations to prioritize clinically relevant variants.

6. For RNA-seq differential expression analysis, quantify transcript abundance with Salmon or kallisto (alignment-free) or featureCounts (alignment-based), normalize with DESeq2 or edgeR, apply multiple testing correction (Benjamini-Hochberg), and validate with volcano plots and MA plots.

7. Orchestrate bioinformatics pipelines using Nextflow or Snakemake for reproducibility; define each step as an isolated process with explicit inputs/outputs, use containers (Docker/Singularity) for tool versioning, and enable resume-on-failure to avoid recomputing completed steps.

8. Store genomic data efficiently: use CRAM instead of BAM for 40-60% compression savings, compress VCF to VCF.gz with bgzip and index with tabix, store FASTA with samtools faidx indexing, and maintain checksums (MD5) for all files to verify data integrity.

9. Build genome assemblies from long reads (PacBio HiFi or Oxford Nanopore) using assemblers like hifiasm or Flye; polish with short reads if needed, evaluate assembly quality with QUAST and BUSCO completeness scores, and scaffold with Hi-C data for chromosome-level assemblies.

10. Construct phylogenetic trees by aligning sequences with MAFFT or MUSCLE, selecting a substitution model with ModelTest-NG, and inferring the tree with RAxML-NG (maximum likelihood) or BEAST (Bayesian); always bootstrap (>= 1000 replicates) to assess branch support confidence.

11. Comply with clinical genomics regulations (CLIA, CAP, HIPAA) by implementing audit trails for all analysis steps, version-locking pipeline dependencies, validating against truth sets (Genome in a Bottle), and maintaining chain-of-custody documentation from sample receipt through reporting.

12. Implement protein structure prediction workflows by integrating AlphaFold2 or ESMFold; pre-compute multiple sequence alignments with MMseqs2, manage GPU resources for inference, and validate predictions against experimental structures using RMSD and GDT scores when available.

13. Integrate BLAST searches by building local databases with `makeblastdb`, using appropriate programs (blastn for nucleotide, blastp for protein, blastx for translated), filtering results by e-value (< 1e-5) and query coverage, and parallelizing large searches across multiple cores.

14. Design genomic data storage schemas that separate metadata (sample info, run parameters) from binary data (BAM/CRAM/VCF); use cloud object storage (S3) with lifecycle policies for archival, implement access controls per dataset, and maintain a data catalog with searchable annotations.
