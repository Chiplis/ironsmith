import * as THREE from 'three';
import { createTablePerspective } from './table-perspective.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { playerBoardLights } from './player-board-lights.js';
import { createZoneSanctuary } from './zone-sanctuary.js';
import { placeForgeScenery } from './forge-scenery.js';

const ASSETS = `${import.meta.env.BASE_URL}theme/forge-arena/`;
const vertexShader = 'varying vec2 vUv; void main(){vUv=uv;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);}';
const noiseShader = `
  float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
  float noise(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);return mix(mix(hash(i),hash(i+vec2(1.,0.)),f.x),mix(hash(i+vec2(0.,1.)),hash(i+vec2(1.)),f.x),f.y);}
  float fbm(vec2 p){return noise(p)*.55+noise(p*2.03)*.28+noise(p*4.11)*.17;}
`;

export function createForgeScene(host) {
  const renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true, powerPreference: 'low-power' });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.5));
  renderer.setClearColor(0x111a22, 0);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  renderer.shadowMap.autoUpdate = false;
  renderer.shadowMap.needsUpdate = true;
  host.append(renderer.domElement);
  const scene = new THREE.Scene();
  const projection = createTablePerspective();
  const { camera } = projection;
  host.dataset.view = "perspective";
  scene.add(new THREE.HemisphereLight(0x7394c6, 0x080b17, 0.35));
  const daylight = new THREE.DirectionalLight(0xaecbff, 1.8);
  daylight.position.set(-300, 450, 700);
  daylight.castShadow = true;
  daylight.shadow.mapSize.set(1024, 1024);
  daylight.shadow.normalBias = 1;
  daylight.shadow.bias = -.0002;
  scene.add(daylight, daylight.target);
  const seatLights = new Map();
  let seatLayout = { width: 1, height: 1, seats: [] }, lightConfiguration = '';
  function illuminatePlayers(signal) {
    const specifications = playerBoardLights(seatLayout, signal.players, signal.perspective);
    const configuration = JSON.stringify(specifications);
    if (configuration === lightConfiguration) return;
    lightConfiguration = configuration;
    const active = new Set(specifications.map(spec => spec.owner));
    for (const [owner, light] of seatLights) if (!active.has(owner)) {
      scene.remove(light, light.target); light.dispose(); seatLights.delete(owner);
    }
    for (const spec of specifications) {
      let light = seatLights.get(spec.owner);
      if (!light) {
        light = new THREE.SpotLight();
        light.penumbra = 1; light.decay = 2; light.castShadow = true;
        light.shadow.mapSize.set(512, 512); light.shadow.bias = -.0002;
        light.shadow.normalBias = .6;
        seatLights.set(spec.owner, light); scene.add(light, light.target);
      }
      light.color.set(spec.color);
      light.position.copy(projection.point(spec.x, spec.y)); light.position.z = spec.z;
      light.target.position.copy(projection.point(spec.targetX, spec.targetY, spec.targetZ));
      light.intensity = spec.intensity; light.distance = spec.distance; light.angle = spec.angle;
      light.shadow.camera.near = 1; light.shadow.camera.far = spec.distance;
      light.shadow.camera.updateProjectionMatrix();
    }
    renderer.shadowMap.needsUpdate = true;
    host.dataset.playerLights = specifications.map(spec => `${spec.owner}:${spec.color}`).join(',');
  }
  const resources = new Set();
  let disposed = false, lost = false, width = 1, height = 1, combat = false;
  let pulseAt = -Infinity, previousTime = 0, placements = [], shadowLayout = "";
  const own = resource => { resources.add(resource); return resource; };
  const plane = own(new THREE.PlaneGeometry(1, 1));
  const ready = () => { if (!disposed) { renderer.shadowMap.needsUpdate = true; host.dispatchEvent(new Event('forgeassetsready')); } };
  function rememberModel(group) {
    group.traverse(object => {
      if (object.geometry) resources.add(object.geometry);
      for (const material of Array.isArray(object.material) ? object.material : object.material ? [object.material] : []) {
        resources.add(material);
        for (const value of Object.values(material)) if (value?.isTexture) resources.add(value);
      }
    });
  }
  // The terrain is an albedo map on a rough, shadow-receiving surface. Light
  // transport, material response and geometric occlusion determine illumination.
  const floorMaterial = own(new THREE.MeshStandardMaterial({
    color: 0xa4aab5, roughness: .97, metalness: 0,
  }));
  const floor = new THREE.Mesh(plane, floorMaterial);
  floor.position.z = -60; floor.receiveShadow = true; floor.visible = false; scene.add(floor);
  new THREE.TextureLoader().load(`${ASSETS}terrain.png`, texture => {
    if (disposed) { texture.dispose(); return; }
    own(texture); texture.colorSpace = THREE.SRGBColorSpace;
    texture.anisotropy = Math.min(4, renderer.capabilities.getMaxAnisotropy());
    floorMaterial.map = texture; floorMaterial.needsUpdate = true; floor.visible = true;
    host.dataset.terrain = 'ready'; ready();
  }, undefined, () => { if (!disposed) host.dataset.terrain = 'fallback'; });

  // Feathered contact shading: retain the landscape instead of enclosing each
  // card in an opaque rectangular platform.
  const shadowMaterial = own(new THREE.ShaderMaterial({
    transparent: true, depthWrite: false, vertexShader,
    fragmentShader: `varying vec2 vUv;void main(){vec2 q=abs(vUv-.5)*2.;float a=1.-smoothstep(.35,1.,max(q.x,q.y));gl_FragColor=vec4(.015,.022,.025,a*.28);}`,
  }));
  const sanctuary = createZoneSanctuary(scene, own);
  // Shadow-only geometry tracks the DOM cards without repainting their artwork.
  // Cards sit above the stone; their shadows share the scenery's moon direction.
  const cardShadowGeometry = own(new THREE.BoxGeometry(1, 1, 1));
  const cardShadowMaterial = own(new THREE.MeshBasicMaterial({ colorWrite: false, depthWrite: false }));
  const cardShadows = [];

  const props = Array.from({ length: 6 }, (_, index) => {
    const group = new THREE.Group(); scene.add(group);
    const shadow = new THREE.Mesh(plane, shadowMaterial); scene.add(shadow);
    return { group, shadow, index, kind: index === 2 || index === 3 ? 'pit' : 'rock', ready: false, size: 0 };
  });
  const loader = new GLTFLoader();
  for (const [kind, file] of [['rock', 'rock_face_02/rock_face_02.gltf'], ['pit', 'stone_fire_pit/stone_fire_pit.gltf']]) {
    loader.load(`${ASSETS}${file}`, gltf => {
      gltf.scene.traverse(object => {
        if (object.isMesh) { object.castShadow = true; object.receiveShadow = true; }
        for (const material of Array.isArray(object.material) ? object.material : object.material ? [object.material] : []) {
          material.color?.multiply(new THREE.Color(kind === 'rock' ? 0x657c94 : 0x8d969e));
          material.roughness = 0.95;
        }
      });
      rememberModel(gltf.scene);
      if (disposed) { resources.forEach(r => r.dispose()); return; }
      for (const prop of props.filter(p => p.kind === kind)) {
        const model = gltf.scene.clone(true);
        model.rotation.x = Math.PI / 3;
        model.rotation.z = kind === 'rock' ? (prop.index % 2 ? -0.65 : 0.65) : (prop.index % 2 ? -0.25 : 0.22);
        model.updateMatrixWorld(true);
        const bounds = new THREE.Box3().setFromObject(model);
        const size = bounds.getSize(new THREE.Vector3());
        const center = bounds.getCenter(new THREE.Vector3());
        const normalizer = new THREE.Group();
        model.position.sub(center); normalizer.add(model);
        normalizer.scale.setScalar(1 / Math.max(size.x, size.y, size.z));
        prop.group.add(normalizer); prop.ready = true;
      }
      host.dataset.models = String(props.filter(p => p.ready).length); ready();
    }, undefined, () => { if (!disposed) host.dataset.modelFallback = 'true'; });
  }

  const flames = props.filter(p => p.kind === 'pit').map(prop => {
    const material = own(new THREE.ShaderMaterial({
      uniforms: { time: { value: 0 }, power: { value: 1 } }, transparent: true, depthWrite: false,
      blending: THREE.AdditiveBlending, vertexShader,
      fragmentShader: `varying vec2 vUv; uniform float time,power; ${noiseShader}
        void main(){vec2 p=(vUv-.5)*2.;float r=length(p);
          float ring=pow(.5+.5*sin(r*30.-time*.7),5.)*.16;
          float a=(1.-smoothstep(.5,1.,r))*(.18+ring);
          gl_FragColor=vec4(.24,.58,1.,a*power);}`,
    }));
    const mesh = new THREE.Mesh(plane, material); scene.add(mesh);
    const light = new THREE.PointLight(0x93c9ff, 0, 250, 1.5); scene.add(light);
    return { prop, mesh, material, light };
  });
  const particleCount = 100;
  const sparkGeometry = own(new THREE.BufferGeometry());
  const sparkPositions = new Float32Array(particleCount * 3);
  sparkGeometry.setAttribute('position', new THREE.BufferAttribute(sparkPositions, 3));
  const sparkMaterial = own(new THREE.ShaderMaterial({
    transparent: true, depthWrite: false, blending: THREE.AdditiveBlending,
    vertexShader: 'attribute float size; varying float strength; void main(){strength=size/4.;gl_PointSize=size;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);}',
    fragmentShader: 'varying float strength;void main(){float a=1.-smoothstep(.0,.5,length(gl_PointCoord-.5));gl_FragColor=vec4(.66,.82,1.,a*strength*.65);}',
  }));
  sparkGeometry.setAttribute('size', new THREE.BufferAttribute(Float32Array.from({ length: particleCount }, (_, i) => 1.5 + (i % 5) * 0.55), 1));
  const sparks = new THREE.Points(sparkGeometry, sparkMaterial); sparks.frustumCulled = false; scene.add(sparks);
  const onLost = event => { event.preventDefault(); lost = true; host.dataset.renderer = 'fallback'; renderer.domElement.style.visibility = 'hidden'; };
  const onRestored = () => { lost = false; host.dataset.renderer = 'webgl'; renderer.domElement.style.visibility = ''; };
  renderer.domElement.addEventListener('webglcontextlost', onLost);
  renderer.domElement.addEventListener('webglcontextrestored', onRestored);
  host.dataset.renderer = 'webgl';
  return {
    layout(layout, zones) {
      seatLayout = layout;
      if (width !== layout.width || height !== layout.height) {
        width = Math.max(1, layout.width); height = Math.max(1, layout.height);
        renderer.setSize(width, height); projection.resize(width, height);
        const terrain = projection.rect({left: -20, top: -20, right: width + 20, bottom: height + 20});
        floor.position.set((terrain.left + terrain.right) / 2, -(terrain.top + terrain.bottom) / 2, -60);
        floor.scale.set(terrain.right - terrain.left, terrain.bottom - terrain.top, 1);
        lightConfiguration = '';
        daylight.position.set(width * .15, height * .25, 750);
        daylight.target.position.set(width * .55, -height * .65, -60);
        const extent = Math.max(width, height) * .85;
        Object.assign(daylight.shadow.camera, { left: -extent, right: extent, top: extent, bottom: -extent, near: 1, far: 3500 });
        daylight.shadow.camera.updateProjectionMatrix();
      }
      const nextShadowLayout = JSON.stringify([layout.width, layout.height, layout.cardShadows, [...zones.values()].map(z => z.rect)]);
      if (nextShadowLayout !== shadowLayout) { renderer.shadowMap.needsUpdate = true; shadowLayout = nextShadowLayout; }
      sanctuary.layout(new Map([...zones].map(([key, zone]) => [key, { ...zone, rect: projection.rect(zone.rect, -22) }])));
      const footprints = layout.cardShadows || [];
      while (cardShadows.length < footprints.length) {
        const mesh = new THREE.Mesh(cardShadowGeometry, cardShadowMaterial);
        mesh.castShadow = true; scene.add(mesh); cardShadows.push(mesh);
      }
      cardShadows.forEach((mesh, index) => {
        const footprint = footprints[index]; mesh.visible = Boolean(footprint);
        if (!footprint) return;
        const r = projection.rect(footprint, -54);
        mesh.position.set((r.left + r.right) / 2, -(r.top + r.bottom) / 2, -54);
        mesh.scale.set(r.right - r.left, r.bottom - r.top, 1);
      });
      placements = placeForgeScenery(width, height, [...layout.obstacles, ...[...zones.values()].map(z => z.rect)]);
    },
    signal(next, pulse, now) { illuminatePlayers(next); combat = next.combat; if (pulse) pulseAt = now; },
    pulse(now) { pulseAt = now; },
    interact(x, y, now) {
      const hit = props.some(prop => prop.group.visible && Math.hypot(x - (prop.screenX ?? 0), y - (prop.screenY ?? 0)) < prop.size / 2);
      if (hit) pulseAt = now;
      return hit;
    },
    draw(now, reducedMotion) {
      if (lost || disposed) return;
      const dt = Math.min(0.1, Math.max(0, (now - previousTime) / 1000)); previousTime = now;
      const t = reducedMotion ? 0 : now / 1000;
      const pulse = reducedMotion ? 0 : Math.max(0, 1 - (now - pulseAt) / 1400);
      daylight.intensity = 1.8 + pulse * .08 + (combat ? .035 : 0);
      props.forEach((prop, index) => {
        const target = placements[index]; if (!target) return;
        // Retreat immediately; restore scenery gradually as space becomes free.
        if (Math.abs(target.size - prop.size) > 1 || target.x !== prop.screenX || target.y !== prop.screenY) renderer.shadowMap.needsUpdate = true;
        prop.size = target.size < prop.size || reducedMotion ? target.size : THREE.MathUtils.damp(prop.size, target.size, 5, dt);
        prop.group.visible = prop.ready && prop.size > 26;
        prop.screenX = target.x; prop.screenY = target.y;
        const center = projection.point(target.x, target.y, 10);
        const edge = projection.point(target.x + 1, target.y, 10);
        const worldSize = prop.size * center.distanceTo(edge);
        prop.group.position.copy(center);
        prop.group.scale.setScalar(worldSize);
        prop.shadow.visible = prop.group.visible;
        prop.shadow.position.copy(projection.point(target.x + prop.size * .1, target.y + prop.size * .12, -35));
        prop.shadow.scale.set(worldSize * 1.7, worldSize * 1.3, 1);
      });
      flames.forEach(({ prop, mesh, material, light }) => {
        mesh.visible = prop.group.visible;
        mesh.position.set(prop.group.position.x, prop.group.position.y, prop.size * 0.55 + 10);
        mesh.scale.set(prop.size * 0.55, prop.size * 0.4, 1);
        material.uniforms.time.value = t; material.uniforms.power.value = 1 + pulse * 0.6;
        light.position.set(prop.group.position.x, prop.group.position.y, prop.size * 0.5 + 10);
        light.intensity = prop.group.visible ? 18 + pulse * 18 : 0;
      });
      sanctuary.draw(t, pulse);
      sparks.visible = !reducedMotion;
      for (let i = 0; i < particleCount; i++) {
        const age = (t * (0.09 + (i % 3) * 0.025) + i * 0.618) % 1;
        const side = i % 2, edge = side ? width * 0.96 : width * 0.04;
        sparkPositions[i * 3] = edge + Math.sin(i * 12.3 + age * 4) * width * .025;
        sparkPositions[i * 3 + 1] = -height * (1 - age) + Math.cos(i * 3.1) * 50;
        sparkPositions[i * 3 + 2] = 120;
      }
      sparkGeometry.attributes.position.needsUpdate = true;
      renderer.render(scene, camera);
    },
    dispose() {
      disposed = true;
      renderer.domElement.removeEventListener('webglcontextlost', onLost);
      renderer.domElement.removeEventListener('webglcontextrestored', onRestored);
      seatLights.forEach(light => light.dispose());
      daylight.dispose();
      resources.forEach(resource => resource.dispose());
      renderer.dispose(); renderer.forceContextLoss(); renderer.domElement.remove();
    },
  };
}
