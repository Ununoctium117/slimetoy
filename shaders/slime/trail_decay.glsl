#version 450

layout(local_size_x = 16, local_size_y = 1, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform image2D img;

layout(push_constant) uniform Constants {
    float decay_speed;
    float diffuse_speed;
} constants;

void main() {
    vec4 orig_color = imageLoad(img, ivec2(gl_GlobalInvocationID.xy));
    vec2 imgS = imageSize(img);

    // blur as well
    vec4 sum = vec4(0);
    for (int i = -1; i <= 1; i++) {
        for (int j = -1; j <= 1; j++) {
            ivec2 samplePos = ivec2(gl_GlobalInvocationID.xy) + ivec2(i, j);
            if (samplePos.x >= 0 && samplePos.x < imgS.x && samplePos.y >= 0 && samplePos.y < imgS.y) {
                sum += imageLoad(img, samplePos);
            }
        }
    }

    vec4 diffused_value = mix(orig_color, sum / 9, constants.diffuse_speed);
    vec4 blurred_decayed_color = max(vec4(0), diffused_value - vec4(constants.decay_speed));

    imageStore(img, ivec2(gl_GlobalInvocationID.xy), blurred_decayed_color);
}