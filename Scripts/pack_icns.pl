#!/usr/bin/env perl
use strict;
use warnings;

@ARGV == 2 or die "Usage: pack_icns.pl ICONSET_DIR OUTPUT.icns\n";
my ($iconset_dir, $output_path) = @ARGV;

# Modern PNG-backed ICNS entries. Duplicate pixel sizes represent different
# point-size/scale-factor combinations and match iconutil's normal output.
my @entries = (
    ["icp4", "icon_16x16.png"],
    ["ic11", "icon_16x16\@2x.png"],
    ["icp5", "icon_32x32.png"],
    ["ic12", "icon_32x32\@2x.png"],
    ["ic07", "icon_128x128.png"],
    ["ic13", "icon_128x128\@2x.png"],
    ["ic08", "icon_256x256.png"],
    ["ic14", "icon_256x256\@2x.png"],
    ["ic09", "icon_512x512.png"],
    ["ic10", "icon_512x512\@2x.png"],
);

my $chunks = "";
for my $entry (@entries) {
    my ($type, $filename) = @$entry;
    my $path = "$iconset_dir/$filename";

    open my $input, "<:raw", $path or die "Unable to read $path: $!\n";
    local $/;
    my $image = <$input>;
    close $input or die "Unable to close $path: $!\n";

    length($image) <= 0xfffffff7
        or die "Icon entry is too large: $filename\n";
    $chunks .= $type . pack("N", length($image) + 8) . $image;
}

length($chunks) <= 0xfffffff7 or die "ICNS output is too large\n";
my $output = "icns" . pack("N", length($chunks) + 8) . $chunks;
my $temporary_path = "$output_path.tmp.$$";

open my $file, ">:raw", $temporary_path
    or die "Unable to create $temporary_path: $!\n";
print {$file} $output or die "Unable to write $temporary_path: $!\n";
close $file or die "Unable to close $temporary_path: $!\n";
rename $temporary_path, $output_path
    or die "Unable to move $temporary_path to $output_path: $!\n";
