import { PageTitle } from '@/components/typography';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from './ui/card';

type Props = {
  title: string;
  description?: string;
  children: React.ReactNode;
};

export const AuthCard = ({ title, description, children }: Props) => {
  return (
    <Card className="wrap-anywhere [--card-spacing:24px] w-full max-w-md">
      <CardHeader>
        <CardTitle>
          <PageTitle className="mb-0 leading-normal">{title}</PageTitle>
        </CardTitle>
        {description && <CardDescription>{description}</CardDescription>}
      </CardHeader>

      <CardContent>{children}</CardContent>
    </Card>
  );
};
