#version 450

#define PI 3.14159265358979323846

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;

layout(push_constant) uniform Data {
    vec2 noise_center;
    // position in complex space of the center of the screen
    vec2 mandelbrot_center;
    // width and height of the viewport in complex space
    vec2 mandelbrot_viewport;
} Constants;

vec3 hsv2rgb(vec3 c) {
    vec4 k = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + k.xyz) * 6.0 - k.www);
    return c.z * mix(k.xxx, clamp(p - k.xxx, 0.0, 1.0), c.y);
}

float rand(vec2 c) {
    return fract(sin(dot(c.xy, vec2(12.9898, 78.233))) * 43758.5453);
}

float noise(vec2 p, float freq) {
    float unit = imageSize(img).x / freq;
    vec2 ij = floor(p / unit);
    vec2 xy = mod(p, unit) / unit;
    xy = 0.5 * (1.0 - cos(PI * xy));

    float a = rand(ij + vec2(0, 0));
    float b = rand(ij + vec2(1, 0));
    float c = rand(ij + vec2(0, 1));
    float d = rand(ij + vec2(1, 1));
    float x1 = mix(a, b, xy.x);
    float x2 = mix(c, d, xy.x);
    return mix(x1, x2, xy.y);
}

float pNoise(vec2 pos, int res) {
    float persistence = 0.5;
    float n = 0;
    float normK = 0;
    float f = 4;
    float amp = 1;

    for (int i = 0; i < res; i++) {
        n += amp * noise(pos, f);
        f *= 2;
        normK += amp;
        amp *= persistence;
    }

    float nf = n / normK;
    return nf * nf * nf * nf;
}

void main() {
    // normalize screen coordinates to values between 0 and 1
    vec2 norm_coordinates = (gl_GlobalInvocationID.xy + vec2(0.5)) / vec2(imageSize(img));
    vec2 c = (norm_coordinates - vec2(0.5)) * 2.0 - Constants.mandelbrot_center;

    // vec2 c = (norm_coordinates - vec2(0.5)) * 2.0 - vec2(1.0, 0.0);

    vec2 z = vec2(0.0, 0.0);
    float i;
    for (i = 0.0; i < 1.0; i += 0.005) {
        z = vec2(
            z.x * z.x - z.y * z.y + c.x,
            z.y * z.x + z.x * z.y + c.y
        );

        if (length(z) > 4.0) {
            break;
        }
    }

    float noise = pNoise(gl_GlobalInvocationID.xy + Constants.noise_center, 2);

    vec4 to_write = vec4(hsv2rgb(vec3(noise, 1.0 - i, 1.0 - i)), 1.0);
    imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
}