#version 450

#define PI 3.14159265358979323846

layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform image2D img;

struct Agent {
    vec2 position;
    float angle;
    float age;
};

layout(set = 1, binding = 0) buffer Data {
    Agent agents[];
} agents;

layout(push_constant) uniform Constants {
    float move_speed;
    float sensor_distance;
    float sensor_angle;
    float turn_speed;
    int sensor_size;
} constants;

float rand01(float inp) {
    inp = fract(inp * 0.1031);
    inp *= inp + 33.33;
    inp *= inp + inp;
    return fract(inp);
}

float sense(Agent agent, float directionOffset) {
    float angle = agent.angle + directionOffset;
    vec2 sensorDirection = vec2(cos(angle), sin(angle));
    vec2 center = agent.position + (sensorDirection * constants.sensor_distance);

    vec2 imgS = imageSize(img);

    float sum = 0;
    for (int offsetX = -constants.sensor_size; offsetX <= constants.sensor_size; offsetX++) {
        for (int offsetY = -constants.sensor_size; offsetY <= constants.sensor_size; offsetY++) {
            ivec2 pos = ivec2(center + vec2(offsetX, offsetY));

            // if (pos.x >= 0 && pos.x < imgS.x && pos.y >= 0 && pos.y < imgS.y) {
            //     vec4 color = imageLoad(img, pos);
            //     sum += color.x + color.y + color.z;
            // }

            if (pos.x < 0) {
                pos.x += int(imgS.x);
            } else if (pos.x >= imgS.x) {
                pos.x -= int(imgS.x);
            }

            if (pos.y < 0) {
                pos.y += int(imgS.y);
            } else if (pos.y >= imgS.y) {
                pos.y -= int(imgS.y);
            }

            vec4 color = imageLoad(img, pos);
            sum += color.x + color.y + color.z;
        }
    }

    return sum;
}

vec3 hsv2rgb(vec3 c) {
    vec4 k = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + k.xyz) * 6.0 - k.www);
    return c.z * mix(k.xxx, clamp(p - k.xxx, 0.0, 1.0), c.y);
}

void main() {
    uint idx = gl_GlobalInvocationID.x;
    if (idx >= agents.agents.length()) { return; }

    Agent agent = agents.agents[gl_GlobalInvocationID.x];

    vec2 imgS = imageSize(img);
    float rand = rand01(idx + agent.position.y * imgS.x + agent.position.x);

    float weightForward = sense(agent, 0);
    float weightLeft = sense(agent, constants.sensor_angle);
    float weightRight = sense(agent, -constants.sensor_angle);

    if (weightForward > weightLeft && weightForward > weightRight) {
        // continue in the same direction
    } else if (weightForward < weightLeft && weightForward < weightRight) {
        // turn randomly
        agents.agents[idx].angle += (rand - 0.5) * 2 * constants.turn_speed;
    } else if (weightRight > weightLeft) {
        // turn right
        agents.agents[idx].angle -= rand * constants.turn_speed;
    } else {
        // turn left
        agents.agents[idx].angle += rand * constants.turn_speed;
    }

    // move based on direction and speed
    vec2 direction = vec2(cos(agent.angle), sin(agent.angle));
    vec2 new_pos = agent.position + (direction * constants.move_speed);

    if (new_pos.x < 0) {
        new_pos.x += imgS.x - 0.01;
    } else if (new_pos.x >= imgS.x) {
        new_pos.x -= imgS.x - 0.01;
    }

    if (new_pos.y < 0) {
        new_pos.y += imgS.y - 0.01;
    } else if (new_pos.y >= imgS.y) {
        new_pos.y -= imgS.y - 0.01;
    }

    // if (new_pos.x < 0 || new_pos.x >= imgS.x || new_pos.y < 0 || new_pos.y >= imgS.y) {
    //     new_pos.x = min(imgS.x - 0.01, max(0, new_pos.x));
    //     new_pos.y = min(imgS.y - 0.01, max(0, new_pos.y));
    //     agents.agents[idx].angle = rand * 2 * PI;
    // }

    agents.agents[idx].position = new_pos;
    agents.agents[idx].age += 0.0005;

    vec4 color = vec4(hsv2rgb(vec3(0.5 * (1 + sin(agents.agents[idx].age)), 0.7, 1)), 1);
    // vec4 color = vec4(0.2, 0.55, 0.8, 1);
    imageStore(img, ivec2(new_pos.x, new_pos.y), color);
}