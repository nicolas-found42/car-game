"""Retain the baseline's qualified DB9/manual and XJ220 upper-bound comparisons."""
import sys
sys.path.insert(0, 'tools')
import compare_fleet40_references as reference
reference.FIELDS.update({'acceleration_0_60_mph_upper_bound':'zero_60_mph_s',
                         'reference_2006_acceleration_0_100_kmh_manual':'zero_100_kmh_s'})
reference.main()
