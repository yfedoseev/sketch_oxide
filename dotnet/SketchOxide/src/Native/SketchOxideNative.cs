using System.Runtime.InteropServices;

namespace SketchOxide.Native;

/// <summary>
/// P/Invoke declarations for all native SketchOxide functions.
/// Maps C# calls to the native Rust library via DLL imports.
/// </summary>
internal static class SketchOxideNative
{
    private const string LibName = "sketch_oxide_dotnet";

    static SketchOxideNative()
    {
        NativeLibraryLoader.Initialize();
    }

    #region Cardinality - HyperLogLog

    [DllImport(LibName)]
    internal static extern nuint hyperloglog_new(uint precision);

    [DllImport(LibName)]
    internal static extern void hyperloglog_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern unsafe void hyperloglog_update(nuint ptr, byte* data, ulong len);

    [DllImport(LibName)]
    internal static extern double hyperloglog_estimate(nuint ptr);

    [DllImport(LibName)]
    internal static extern void hyperloglog_merge(nuint ptr1, nuint ptr2);

    [DllImport(LibName)]
    internal static extern byte[] hyperloglog_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint hyperloglog_deserialize(byte[] data, ulong len);

    [DllImport(LibName)]
    internal static extern uint hyperloglog_precision(nuint ptr);

    #endregion

    #region Cardinality - UltraLogLog

    [DllImport(LibName)]
    internal static extern nuint ultraloglog_new(uint precision);

    [DllImport(LibName)]
    internal static extern void ultraloglog_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern unsafe void ultraloglog_update(nuint ptr, byte* data, ulong len);

    [DllImport(LibName)]
    internal static extern double ultraloglog_estimate(nuint ptr);

    [DllImport(LibName)]
    internal static extern void ultraloglog_merge(nuint ptr1, nuint ptr2);

    [DllImport(LibName)]
    internal static extern byte[] ultraloglog_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint ultraloglog_deserialize(byte[] data, ulong len);

    #endregion

    #region Cardinality - CpcSketch

    [DllImport(LibName)]
    internal static extern nuint cpc_new(uint lgk);

    [DllImport(LibName)]
    internal static extern void cpc_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void cpc_update(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern double cpc_estimate(nuint ptr);

    [DllImport(LibName)]
    internal static extern void cpc_merge(nuint ptr1, nuint ptr2);

    [DllImport(LibName)]
    internal static extern byte[] cpc_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint cpc_deserialize(byte[] data, ulong len);

    #endregion

    #region Cardinality - QSketch

    [DllImport(LibName)]
    internal static extern nuint qsketch_new(uint max_samples);

    [DllImport(LibName)]
    internal static extern void qsketch_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void qsketch_update(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern void qsketch_update_weighted(nuint ptr, byte[] item, ulong len, double weight);

    [DllImport(LibName)]
    internal static extern double qsketch_estimate(nuint ptr);

    [DllImport(LibName)]
    internal static extern void qsketch_merge(nuint ptr1, nuint ptr2);

    [DllImport(LibName)]
    internal static extern byte[] qsketch_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint qsketch_deserialize(byte[] data, ulong len);

    #endregion

    #region Cardinality - ThetaSketch

    [DllImport(LibName)]
    internal static extern nuint theta_new(uint lgk);

    [DllImport(LibName)]
    internal static extern void theta_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void theta_update(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern double theta_estimate(nuint ptr);

    [DllImport(LibName)]
    internal static extern void theta_merge(nuint ptr1, nuint ptr2);

    [DllImport(LibName)]
    internal static extern byte[] theta_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint theta_deserialize(byte[] data, ulong len);

    #endregion

    #region Frequency - CountMinSketch

    [DllImport(LibName)]
    internal static extern nuint countmin_new(double epsilon, double delta);

    [DllImport(LibName)]
    internal static extern void countmin_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void countmin_update(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern ulong countmin_estimate(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern void countmin_merge(nuint ptr1, nuint ptr2);

    [DllImport(LibName)]
    internal static extern byte[] countmin_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint countmin_deserialize(byte[] data, ulong len);

    #endregion

    #region Frequency - CountSketch

    [DllImport(LibName)]
    internal static extern nuint countsketch_new(uint width, uint depth);

    [DllImport(LibName)]
    internal static extern void countsketch_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void countsketch_update(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern double countsketch_estimate(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern void countsketch_merge(nuint ptr1, nuint ptr2);

    [DllImport(LibName)]
    internal static extern byte[] countsketch_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint countsketch_deserialize(byte[] data, ulong len);

    #endregion

    #region Frequency - ConservativeCountMin

    [DllImport(LibName)]
    internal static extern nuint conservativecountmin_new(double epsilon, double delta);

    [DllImport(LibName)]
    internal static extern void conservativecountmin_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void conservativecountmin_update(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern ulong conservativecountmin_estimate(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern void conservativecountmin_merge(nuint ptr1, nuint ptr2);

    [DllImport(LibName)]
    internal static extern byte[] conservativecountmin_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint conservativecountmin_deserialize(byte[] data, ulong len);

    #endregion

    #region Frequency - SpaceSaving

    [DllImport(LibName)]
    internal static extern nuint spacesaving_new(uint k);

    [DllImport(LibName)]
    internal static extern void spacesaving_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void spacesaving_update(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern byte[] spacesaving_top_k(nuint ptr, out uint count);

    [DllImport(LibName)]
    internal static extern byte[] spacesaving_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint spacesaving_deserialize(byte[] data, ulong len);

    #endregion

    #region Frequency - FrequentItems

    [DllImport(LibName)]
    internal static extern nuint frequentitems_new(double error);

    [DllImport(LibName)]
    internal static extern void frequentitems_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void frequentitems_update(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern byte[] frequentitems_query(nuint ptr, byte[] item, ulong len, out ulong lower, out ulong upper);

    [DllImport(LibName)]
    internal static extern byte[] frequentitems_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint frequentitems_deserialize(byte[] data, ulong len);

    #endregion

    #region Frequency - ElasticSketch

    [DllImport(LibName)]
    internal static extern nuint elasticsketch_new(uint bucket_count, uint depth);

    [DllImport(LibName)]
    internal static extern void elasticsketch_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void elasticsketch_update(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern ulong elasticsketch_estimate(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern byte[] elasticsketch_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint elasticsketch_deserialize(byte[] data, ulong len);

    #endregion

    #region Frequency - SALSA

    [DllImport(LibName)]
    internal static extern nuint salsa_new(double confidence_metric);

    [DllImport(LibName)]
    internal static extern void salsa_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void salsa_update(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern ulong salsa_estimate(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern byte[] salsa_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint salsa_deserialize(byte[] data, ulong len);

    #endregion

    #region Frequency - RemovableUniversalSketch

    [DllImport(LibName)]
    internal static extern nuint russketch_new(double epsilon, double delta);

    [DllImport(LibName)]
    internal static extern void russketch_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void russketch_update(nuint ptr, byte[] item, ulong len, int delta_value);

    [DllImport(LibName)]
    internal static extern double russketch_estimate(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern double russketch_l2_norm(nuint ptr);

    [DllImport(LibName)]
    internal static extern byte[] russketch_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint russketch_deserialize(byte[] data, ulong len);

    #endregion

    #region Membership - BloomFilter

    [DllImport(LibName)]
    internal static extern nuint bloomfilter_new(ulong size, double fpr);

    [DllImport(LibName)]
    internal static extern void bloomfilter_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void bloomfilter_insert(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool bloomfilter_contains(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern byte[] bloomfilter_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint bloomfilter_deserialize(byte[] data, ulong len);

    #endregion

    #region Membership - BlockedBloomFilter

    [DllImport(LibName)]
    internal static extern nuint blockedbloomfilter_new(ulong size, double fpr);

    [DllImport(LibName)]
    internal static extern void blockedbloomfilter_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void blockedbloomfilter_insert(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool blockedbloomfilter_contains(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern byte[] blockedbloomfilter_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint blockedbloomfilter_deserialize(byte[] data, ulong len);

    #endregion

    #region Membership - CountingBloomFilter

    [DllImport(LibName)]
    internal static extern nuint countingbloomfilter_new(ulong size, double fpr);

    [DllImport(LibName)]
    internal static extern void countingbloomfilter_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void countingbloomfilter_insert(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool countingbloomfilter_contains(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern void countingbloomfilter_remove(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern byte[] countingbloomfilter_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint countingbloomfilter_deserialize(byte[] data, ulong len);

    #endregion

    #region Membership - CuckooFilter

    [DllImport(LibName)]
    internal static extern nuint cuckoofilter_new(ulong size);

    [DllImport(LibName)]
    internal static extern void cuckoofilter_free(nuint ptr);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool cuckoofilter_insert(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool cuckoofilter_contains(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool cuckoofilter_remove(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern byte[] cuckoofilter_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint cuckoofilter_deserialize(byte[] data, ulong len);

    #endregion

    #region Membership - BinaryFuseFilter

    [DllImport(LibName)]
    internal static extern nuint binaryfusefilter_new(byte[][] items, ulong item_count, ulong item_len);

    [DllImport(LibName)]
    internal static extern void binaryfusefilter_free(nuint ptr);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool binaryfusefilter_contains(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern byte[] binaryfusefilter_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint binaryfusefilter_deserialize(byte[] data, ulong len);

    #endregion

    #region Membership - RibbonFilter

    [DllImport(LibName)]
    internal static extern nuint ribbonfilter_new(ulong size, double fpr);

    [DllImport(LibName)]
    internal static extern void ribbonfilter_free(nuint ptr);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool ribbonfilter_insert(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool ribbonfilter_build(nuint ptr);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool ribbonfilter_contains(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern byte[] ribbonfilter_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint ribbonfilter_deserialize(byte[] data, ulong len);

    #endregion

    #region Membership - StableBloomFilter

    [DllImport(LibName)]
    internal static extern nuint stablebloomfilter_new(ulong size, double fpr);

    [DllImport(LibName)]
    internal static extern void stablebloomfilter_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void stablebloomfilter_insert(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool stablebloomfilter_contains(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern byte[] stablebloomfilter_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint stablebloomfilter_deserialize(byte[] data, ulong len);

    #endregion

    #region Quantiles - DDSketch

    [DllImport(LibName)]
    internal static extern nuint ddsketch_new(double relative_accuracy);

    [DllImport(LibName)]
    internal static extern void ddsketch_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void ddsketch_update(nuint ptr, double value);

    [DllImport(LibName)]
    internal static extern double ddsketch_quantile(nuint ptr, double q);

    [DllImport(LibName)]
    internal static extern byte[] ddsketch_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint ddsketch_deserialize(byte[] data, ulong len);

    #endregion

    #region Quantiles - ReqSketch

    [DllImport(LibName)]
    internal static extern nuint reqsketch_new(uint k);

    [DllImport(LibName)]
    internal static extern void reqsketch_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void reqsketch_update(nuint ptr, double value);

    [DllImport(LibName)]
    internal static extern double reqsketch_query(nuint ptr, double rank);

    [DllImport(LibName)]
    internal static extern byte[] reqsketch_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint reqsketch_deserialize(byte[] data, ulong len);

    #endregion

    #region Quantiles - TDigest

    [DllImport(LibName)]
    internal static extern nuint tdigest_new(double compression);

    [DllImport(LibName)]
    internal static extern void tdigest_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void tdigest_update(nuint ptr, double value);

    [DllImport(LibName)]
    internal static extern double tdigest_quantile(nuint ptr, double q);

    [DllImport(LibName)]
    internal static extern double tdigest_cdf(nuint ptr, double value);

    [DllImport(LibName)]
    internal static extern byte[] tdigest_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint tdigest_deserialize(byte[] data, ulong len);

    #endregion

    #region Quantiles - KllSketch

    [DllImport(LibName)]
    internal static extern nuint kll_new(uint k);

    [DllImport(LibName)]
    internal static extern void kll_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void kll_update(nuint ptr, double value);

    [DllImport(LibName)]
    internal static extern double kll_query(nuint ptr, double rank);

    [DllImport(LibName)]
    internal static extern byte[] kll_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint kll_deserialize(byte[] data, ulong len);

    #endregion

    #region Quantiles - SplineSketch

    [DllImport(LibName)]
    internal static extern nuint splinesketch_new(uint max_buckets);

    [DllImport(LibName)]
    internal static extern void splinesketch_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void splinesketch_update(nuint ptr, double value);

    [DllImport(LibName)]
    internal static extern double splinesketch_query(nuint ptr, double rank);

    [DllImport(LibName)]
    internal static extern byte[] splinesketch_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint splinesketch_deserialize(byte[] data, ulong len);

    #endregion

    #region Streaming - SlidingWindowCounter

    [DllImport(LibName)]
    internal static extern nuint slidingwindowcounter_new(ulong window_size, double epsilon);

    [DllImport(LibName)]
    internal static extern void slidingwindowcounter_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void slidingwindowcounter_increment(nuint ptr, ulong timestamp);

    [DllImport(LibName)]
    internal static extern void slidingwindowcounter_increment_by(nuint ptr, ulong timestamp, ulong count);

    [DllImport(LibName)]
    internal static extern ulong slidingwindowcounter_count(nuint ptr, ulong start_time, ulong end_time);

    [DllImport(LibName)]
    internal static extern byte[] slidingwindowcounter_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint slidingwindowcounter_deserialize(byte[] data, ulong len);

    #endregion

    #region Streaming - ExponentialHistogram

    [DllImport(LibName)]
    internal static extern nuint exponentialhistogram_new(ulong window_size, double epsilon);

    [DllImport(LibName)]
    internal static extern void exponentialhistogram_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void exponentialhistogram_insert(nuint ptr, ulong timestamp);

    [DllImport(LibName)]
    internal static extern ulong exponentialhistogram_count(nuint ptr, ulong start_time, ulong end_time);

    [DllImport(LibName)]
    internal static extern void exponentialhistogram_expire(nuint ptr, ulong current_time);

    [DllImport(LibName)]
    internal static extern byte[] exponentialhistogram_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint exponentialhistogram_deserialize(byte[] data, ulong len);

    #endregion

    #region Similarity - MinHash

    [DllImport(LibName)]
    internal static extern nuint minhash_new(uint num_perm);

    [DllImport(LibName)]
    internal static extern void minhash_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void minhash_update(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern double minhash_jaccard_similarity(nuint ptr1, nuint ptr2);

    [DllImport(LibName)]
    internal static extern byte[] minhash_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint minhash_deserialize(byte[] data, ulong len);

    #endregion

    #region Similarity - SimHash

    [DllImport(LibName)]
    internal static extern nuint simhash_new();

    [DllImport(LibName)]
    internal static extern void simhash_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void simhash_update(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern ulong simhash_fingerprint(nuint ptr);

    [DllImport(LibName)]
    internal static extern uint simhash_hamming_distance(ulong fp1, ulong fp2);

    [DllImport(LibName)]
    internal static extern double simhash_similarity(ulong fp1, ulong fp2);

    [DllImport(LibName)]
    internal static extern byte[] simhash_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint simhash_deserialize(byte[] data, ulong len);

    #endregion

    #region Sampling - ReservoirSampling

    [DllImport(LibName)]
    internal static extern nuint reservoirsampling_new(uint k);

    [DllImport(LibName)]
    internal static extern void reservoirsampling_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void reservoirsampling_update(nuint ptr, byte[] item, ulong len);

    [DllImport(LibName)]
    internal static extern byte[] reservoirsampling_sample(nuint ptr, out uint count);

    [DllImport(LibName)]
    internal static extern byte[] reservoirsampling_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint reservoirsampling_deserialize(byte[] data, ulong len);

    #endregion

    #region Sampling - VarOptSampling

    [DllImport(LibName)]
    internal static extern nuint varoptssampling_new(uint k);

    [DllImport(LibName)]
    internal static extern void varoptssampling_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern void varoptssampling_update_weighted(nuint ptr, byte[] item, ulong len, double weight);

    [DllImport(LibName)]
    internal static extern double varoptssampling_total_weight(nuint ptr);

    [DllImport(LibName)]
    internal static extern byte[] varoptssampling_serialize(nuint ptr, out ulong len);

    [DllImport(LibName)]
    internal static extern nuint varoptssampling_deserialize(byte[] data, ulong len);

    #endregion

    #region Frequency - HeavyKeeper

    [DllImport(LibName)]
    internal static extern nuint heavy_keeper_new(uint k, double epsilon, double delta);

    [DllImport(LibName)]
    internal static extern void heavy_keeper_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern unsafe void heavy_keeper_update(nuint ptr, byte* data, ulong len);

    [DllImport(LibName)]
    internal static extern unsafe uint heavy_keeper_estimate(nuint ptr, byte* data, ulong len);

    [DllImport(LibName)]
    internal static extern unsafe ulong heavy_keeper_top_k(nuint ptr, byte* out_buf, ulong buf_size);

    [DllImport(LibName)]
    internal static extern void heavy_keeper_decay(nuint ptr);

    #endregion

    #region Reconciliation - RatelessIBLT

    [DllImport(LibName)]
    internal static extern nuint rateless_iblt_new(ulong expected_diff, ulong cell_size);

    [DllImport(LibName)]
    internal static extern void rateless_iblt_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern unsafe int rateless_iblt_insert(
        nuint ptr, byte* key, ulong key_len, byte* value, ulong value_len);

    [DllImport(LibName)]
    internal static extern unsafe int rateless_iblt_delete(
        nuint ptr, byte* key, ulong key_len, byte* value, ulong value_len);

    [DllImport(LibName)]
    internal static extern int rateless_iblt_subtract(nuint ptr1, nuint ptr2);

    #endregion

    #region Range Filters - Grafite

    [DllImport(LibName)]
    internal static extern unsafe nuint grafite_build(ulong* keys, ulong keys_len, ulong bits_per_key);

    [DllImport(LibName)]
    internal static extern void grafite_free(nuint ptr);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool grafite_may_contain_range(nuint ptr, ulong low, ulong high);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool grafite_may_contain(nuint ptr, ulong key);

    [DllImport(LibName)]
    internal static extern double grafite_expected_fpr(nuint ptr, ulong range_width);

    #endregion

    #region Range Filters - MementoFilter

    [DllImport(LibName)]
    internal static extern nuint memento_filter_new(ulong expected_elements, double fpr);

    [DllImport(LibName)]
    internal static extern void memento_filter_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern unsafe int memento_filter_insert(
        nuint ptr, ulong key, byte* value, ulong value_len);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool memento_filter_may_contain_range(nuint ptr, ulong low, ulong high);

    #endregion

    #region Streaming - SlidingHyperLogLog

    [DllImport(LibName)]
    internal static extern nuint sliding_hll_new(byte precision, ulong max_window_seconds);

    [DllImport(LibName)]
    internal static extern void sliding_hll_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern unsafe int sliding_hll_update(
        nuint ptr, byte* data, ulong len, ulong timestamp);

    [DllImport(LibName)]
    internal static extern double sliding_hll_estimate_window(
        nuint ptr, ulong current_time, ulong window_seconds);

    [DllImport(LibName)]
    internal static extern double sliding_hll_estimate_total(nuint ptr);

    [DllImport(LibName)]
    internal static extern int sliding_hll_decay(
        nuint ptr, ulong current_time, ulong window_seconds);

    #endregion

    #region Membership - VacuumFilter

    [DllImport(LibName)]
    internal static extern nuint vacuum_filter_new(ulong capacity, double fpr);

    [DllImport(LibName)]
    internal static extern void vacuum_filter_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern unsafe int vacuum_filter_insert(nuint ptr, byte* data, ulong len);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern unsafe bool vacuum_filter_contains(nuint ptr, byte* data, ulong len);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern unsafe bool vacuum_filter_delete(nuint ptr, byte* data, ulong len);

    [DllImport(LibName)]
    internal static extern unsafe void vacuum_filter_stats(
        nuint ptr, ulong* out_capacity, ulong* out_num_items, double* out_load_factor, ulong* out_memory_bits);

    [DllImport(LibName)]
    internal static extern void vacuum_filter_clear(nuint ptr);

    #endregion

    #region Range Filters - GRF

    [DllImport(LibName)]
    internal static extern unsafe nuint grf_build(ulong* keys, ulong keys_len, ulong bits_per_key);

    [DllImport(LibName)]
    internal static extern void grf_free(nuint ptr);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool grf_may_contain_range(nuint ptr, ulong low, ulong high);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool grf_may_contain(nuint ptr, ulong key);

    [DllImport(LibName)]
    internal static extern double grf_expected_fpr(nuint ptr, ulong range_width);

    [DllImport(LibName)]
    internal static extern unsafe void grf_stats(
        nuint ptr, ulong* out_key_count, ulong* out_segment_count, double* out_avg_keys_per_segment,
        ulong* out_bits_per_key, ulong* out_total_bits);

    [DllImport(LibName)]
    internal static extern ulong grf_key_count(nuint ptr);

    #endregion

    #region Frequency - NitroSketch

    [DllImport(LibName)]
    internal static extern nuint nitro_sketch_new(double epsilon, double delta, double sample_rate);

    [DllImport(LibName)]
    internal static extern void nitro_sketch_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern unsafe void nitro_sketch_update_sampled(nuint ptr, byte* data, ulong len);

    [DllImport(LibName)]
    internal static extern unsafe uint nitro_sketch_query(nuint ptr, byte* data, ulong len);

    [DllImport(LibName)]
    internal static extern int nitro_sketch_sync(nuint ptr, double sync_ratio);

    [DllImport(LibName)]
    internal static extern unsafe void nitro_sketch_stats(
        nuint ptr, double* out_sample_rate, ulong* out_sampled_count,
        ulong* out_unsampled_count, ulong* out_total_items_estimated);

    #endregion

    #region Universal - UnivMon

    [DllImport(LibName)]
    internal static extern nuint univmon_new(ulong max_stream_size, double epsilon, double delta);

    [DllImport(LibName)]
    internal static extern void univmon_free(nuint ptr);

    [DllImport(LibName)]
    internal static extern unsafe int univmon_update(nuint ptr, byte* data, ulong len, double value);

    [DllImport(LibName)]
    internal static extern double univmon_estimate_l1(nuint ptr);

    [DllImport(LibName)]
    internal static extern double univmon_estimate_l2(nuint ptr);

    [DllImport(LibName)]
    internal static extern double univmon_estimate_entropy(nuint ptr);

    [DllImport(LibName)]
    internal static extern double univmon_detect_change(nuint ptr1, nuint ptr2);

    [DllImport(LibName)]
    internal static extern unsafe void univmon_stats(
        nuint ptr, ulong* out_num_layers, ulong* out_total_memory,
        ulong* out_samples_processed);

    [DllImport(LibName)]
    internal static extern double univmon_epsilon(nuint ptr);

    [DllImport(LibName)]
    internal static extern double univmon_delta(nuint ptr);

    #endregion

    #region Membership - LearnedBloomFilter

    [DllImport(LibName)]
    internal static extern unsafe nuint learned_bloom_new(
        byte** training_keys, ulong* key_lengths, ulong num_keys, double fpr);

    [DllImport(LibName)]
    internal static extern void learned_bloom_free(nuint ptr);

    [DllImport(LibName)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern unsafe bool learned_bloom_contains(nuint ptr, byte* data, ulong len);

    [DllImport(LibName)]
    internal static extern ulong learned_bloom_memory_usage(nuint ptr);

    [DllImport(LibName)]
    internal static extern double learned_bloom_fpr(nuint ptr);

    #endregion
}
