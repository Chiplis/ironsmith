import { effectPixelRatio } from '../../../lib/graphics-quality.js';
import * as THREE from 'three';
import { createTablePerspective } from './table-perspective.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { playerBoardLights } from './player-board-lights.js';
import { createZoneSanctuary } from './zone-sanctuary.js';
import { placeForgeScenery } from './forge-scenery.js';

const ASSETS = `${import.meta.env.BASE_URL}theme/forge-arena/`;

export function createForgeScene(host, quality = 'high') {
  const renderer = new THREE.WebGLRenderer({ alpha: true, antialias: quality === 'high', powerPreference: 'low-power' });
  renderer.setPixelRatio(1);
  renderer.setClearColor(0x111a22, 0);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1;
  renderer.shadowMap.enabled = false;
  host.append(renderer.domElement);
  const scene = new THREE.Scene();
  const projection = createTablePerspective();
  const { camera } = projection;
  host.dataset.view = "perspective";
  scene.add(new THREE.HemisphereLight(0x7394c6, 0x080b17, 0.35));
  const daylight = new THREE.DirectionalLight(0xaecbff, 1.8);
  daylight.position.set(-300, 450, 700);

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
        light.penumbra = 1; light.decay = 2;
        seatLights.set(spec.owner, light); scene.add(light, light.target);
      }
      light.color.set(spec.color);
      light.position.copy(projection.point(spec.x, spec.y)); light.position.z = spec.z;
      light.target.position.copy(projection.point(spec.targetX, spec.targetY, spec.targetZ));
      // Normalize at the tabletop in world space: the far edge is stretched by
      // perspective and receives light at a shallower angle than the near edge.
      const distance = light.position.distanceTo(light.target.position);
      const incidence = (spec.z - spec.targetZ) / distance;
      light.intensity = distance * distance * 1.3 / Math.max(.25, incidence);
      light.distance = distance * 2.6; light.angle = spec.angle;
    }
    host.dataset.playerLights = specifications.map(spec => `${spec.owner}:${spec.color}`).join(',');
  }
  const resources = new Set();
  let disposed = false, lost = false, width = 1, height = 1, pixelRatio = 0;
  let placements = [];
  const own = resource => { resources.add(resource); return resource; };
  const plane = own(new THREE.PlaneGeometry(1, 1));
  const ready = () => { if (!disposed) host.dispatchEvent(new Event('forgeassetsready')); };
  function rememberModel(group) {
    group.traverse(object => {
      if (object.geometry) resources.add(object.geometry);
      for (const material of Array.isArray(object.material) ? object.material : object.material ? [object.material] : []) {
        resources.add(material);
        for (const value of Object.values(material)) if (value?.isTexture) resources.add(value);
      }
    });
  }
  // Sanctuary terrain and stone remain lit by the players, without cast shadows.
  const floorMaterial = own(new THREE.MeshStandardMaterial({
    color: 0xa4aab5, roughness: .97, metalness: 0,
  }));
  const floor = new THREE.Mesh(plane, floorMaterial);
  floor.position.z = -60; floor.visible = false; scene.add(floor);
  new THREE.TextureLoader().load(`${ASSETS}terrain.png`, texture => {
    if (disposed) { texture.dispose(); return; }
    own(texture); texture.colorSpace = THREE.SRGBColorSpace;
    texture.anisotropy = Math.min(4, renderer.capabilities.getMaxAnisotropy());
    floorMaterial.map = texture; floorMaterial.needsUpdate = true; floor.visible = true;
    host.dataset.terrain = 'ready'; ready();
  }, undefined, () => { if (!disposed) host.dataset.terrain = 'fallback'; });

  const sanctuary = createZoneSanctuary(scene, own);

  const props = Array.from({ length: 6 }, (_, index) => {
    const group = new THREE.Group(); scene.add(group);
    return { group, index, kind: index === 2 || index === 3 ? 'pit' : 'rock', ready: false, size: 0 };
  });
  const loader = new GLTFLoader();
  for (const [kind, file] of [['rock', 'rock_face_02/rock_face_02.gltf'], ['pit', 'stone_fire_pit/stone_fire_pit.gltf']]) {
    loader.load(`${ASSETS}${file}`, gltf => {
      gltf.scene.traverse(object => {
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

  const onLost = event => { event.preventDefault(); lost = true; host.dataset.renderer = 'fallback'; renderer.domElement.style.visibility = 'hidden'; };
  const onRestored = () => { lost = false; host.dataset.renderer = 'webgl'; renderer.domElement.style.visibility = ''; };
  renderer.domElement.addEventListener('webglcontextlost', onLost);
  renderer.domElement.addEventListener('webglcontextrestored', onRestored);
  host.dataset.renderer = 'webgl';
  host.dataset.effects = 'player-lights';
  host.dataset.shadows = 'none';
  return {
    layout(layout, zones) {
      seatLayout = layout;
      const nextPixelRatio = effectPixelRatio(layout.width, layout.height, window.devicePixelRatio || 1, quality);
      if (width !== layout.width || height !== layout.height || pixelRatio !== nextPixelRatio) {
        pixelRatio = nextPixelRatio;
        width = Math.max(1, layout.width); height = Math.max(1, layout.height);
        renderer.setPixelRatio(pixelRatio);
        renderer.setSize(width, height); projection.resize(width, height);
        const terrain = projection.rect({left: -20, top: -20, right: width + 20, bottom: height + 20});
        floor.position.set((terrain.left + terrain.right) / 2, -(terrain.top + terrain.bottom) / 2, -60);
        floor.scale.set(terrain.right - terrain.left, terrain.bottom - terrain.top, 1);
        lightConfiguration = '';
        daylight.position.set(width * .15, height * .25, 750);
        daylight.target.position.set(width * .55, -height * .65, -60);
      }
      sanctuary.layout(new Map([...zones].map(([key, zone]) => [key, { ...zone, rect: projection.rect(zone.rect, -22) }])));
      placements = placeForgeScenery(width, height, [...layout.obstacles, ...[...zones.values()].map(z => z.rect)]);
    },
    signal(next) { illuminatePlayers(next); },
    interact() { return false; },
    draw() {
      if (lost || disposed) return;
      props.forEach((prop, index) => {
        const target = placements[index]; if (!target) return;
        // Keep scenery clear of cards without animating the background.
        prop.size = target.size;
        prop.group.visible = prop.ready && prop.size > 26;
        prop.screenX = target.x; prop.screenY = target.y;
        const center = projection.point(target.x, target.y, 10);
        const edge = projection.point(target.x + 1, target.y, 10);
        const worldSize = prop.size * center.distanceTo(edge);
        prop.group.position.copy(center);
        prop.group.scale.setScalar(worldSize);
      });
      renderer.render(scene, camera);
      return false;
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
