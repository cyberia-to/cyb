import { useState } from 'react';
import { Display, Select } from 'src/components';
import { LLMAvatar, modelName } from 'src/containers/Search/LLMSpark/LLMSpark';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import models from './models.json';

type Model = {
  slug: string;
  description: string;
};

function LLMSelect() {
  // const [models, setModels] = useState<Model[]>([]);
  const [currentModel, setCurrentModel] = useState(modelName);

  // useEffect(() => {
  //   (async () => {
  //     try {
  //       const response = await axios.get(
  //         'https://openrouter.ai/api/frontend/models'
  //       );
  //       debugger;
  //       setModels(response.data);
  //     } catch (error) {
  //       console.error(error);
  //     }
  //   })();
  // }, []);

  console.log(models);

  return (
    <Select
      valueSelect={currentModel}
      onChangeSelect={setCurrentModel}
      options={models.map((model) => ({
        value: model.slug,
        text: model.shortName,
      }))}
    />
  );
}

function LLM() {
  useAdviserTexts({
    defaultText: 'LLM setting',
  });
  return (
    <Display>
      LLM
      <LLMAvatar />
      <br />
      <p>selection in work in progress</p>
      <br />
      <LLMSelect />
    </Display>
  );
}

export default LLM;
